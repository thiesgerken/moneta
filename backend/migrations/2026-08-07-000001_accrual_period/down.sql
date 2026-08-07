-- the views select booking_date, so they go before the column does
DROP VIEW expenses_rendered;
DROP VIEW expense_transactions_rendered;

ALTER TABLE expense_transactions DROP COLUMN booking_date;

ALTER TABLE expenses RENAME COLUMN accrual_start TO booking_start;
ALTER TABLE expenses RENAME COLUMN accrual_end TO booking_end;

ALTER TABLE expenses
    RENAME CONSTRAINT expenses_accrual_period_forward TO expenses_booking_period_forward;
ALTER INDEX expenses_accrual_period_idx RENAME TO expenses_booking_period_idx;


CREATE VIEW expense_transactions_rendered AS
SELECT
    t.id AS transaction_id,
    t.expense_id,
    eu.user_id,
    t.idx,
    t.date,
    t.comments,
    t.statement,

    -- a mirrored transaction is reported against the account in the reader's own books
    CASE WHEN s.mirror THEN b.id ELSE a.id END AS account_id,
    CASE WHEN s.mirror AND a.sync_invert THEN -t.amount ELSE t.amount END AS amount,
    CASE WHEN s.mirror AND a.sync_invert THEN -t.fraction ELSE t.fraction END AS fraction,

    c.calculated_amount,

    -- how much the reader's total wealth moved: zero for a transaction on someone else's
    -- account that they do not mirror
    CASE
        WHEN s.mirror THEN b.ownership_fraction * c.calculated_amount
        WHEN a.user_id = eu.user_id THEN a.ownership_fraction * c.calculated_amount
        ELSE 0
    END AS effective_amount
FROM expense_transactions t
JOIN expenses e ON e.id = t.expense_id
JOIN expense_users eu ON eu.expense_id = t.expense_id
JOIN accounts a ON a.id = t.account_id
LEFT JOIN accounts b ON b.id = a.sync_account_id
CROSS JOIN LATERAL (
    SELECT
        (b.id IS NOT NULL AND b.user_id = eu.user_id) AS mirror,
        -- null when a fractional transaction has no reference amount to resolve against.
        -- The Rust side declares this column NOT NULL so such a row fails loudly rather
        -- than reporting a silently wrong figure.
        coalesce(t.amount::double precision, t.fraction * e.reference_amount) AS base
) s
CROSS JOIN LATERAL (
    SELECT CASE WHEN s.mirror AND a.sync_invert THEN -s.base ELSE s.base END
        AS calculated_amount
) c;

CREATE VIEW expenses_rendered AS
SELECT
    -- a reader only ever sees their own rendering, so the upstream id identifies it for them
    e.id AS id,
    e.id AS upstream_id,
    eu.user_id,

    e.title,
    e.store,
    e.description,
    e.comments,
    e.reference_amount,
    e.booking_start,
    e.booking_end,
    e.is_template,
    e.is_tax_relevant,
    e.is_unchecked,
    e.is_incomplete,
    e.is_extraordinary,
    e.receipts,
    e.positions,

    coalesce(cat.categories, '[]'::jsonb) AS categories,

    -- not coalesced: expense_users only yields a row where a transaction exists, so an empty
    -- list here would mean the view is wrong and should surface rather than read as "none"
    tx.transactions,
    tx.effective_amount,

    e.created_at,
    e.updated_at
FROM expenses e
JOIN expense_users eu ON eu.expense_id = e.id
LEFT JOIN LATERAL (
    -- ordered only so the rendered document is stable between reads; categorizations are a
    -- set and nothing downstream depends on which comes first
    SELECT jsonb_agg(jsonb_build_object(
        'categoryId', ec.category_id,
        'weight', ec.weight
    ) ORDER BY ec.category_id) AS categories
    FROM expense_categorizations ec
    WHERE ec.expense_id = e.id
) cat ON true
LEFT JOIN LATERAL (
    SELECT
        jsonb_agg(jsonb_build_object(
            'accountId', r.account_id,
            'date', r.date,
            'amount', r.amount,
            'fraction', r.fraction,
            'comments', r.comments,
            'statement', r.statement,
            'calculatedAmount', r.calculated_amount,
            'effectiveAmount', r.effective_amount
        ) ORDER BY r.idx) AS transactions,

        -- sum() skips nulls, which would quietly drop an unresolvable transaction out of the
        -- total instead of reporting one. Propagate the null so the read fails loudly.
        CASE WHEN bool_or(r.effective_amount IS NULL) THEN NULL
             ELSE sum(r.effective_amount) END AS effective_amount
    FROM expense_transactions_rendered r
    WHERE r.expense_id = e.id AND r.user_id = eu.user_id
) tx ON true;
