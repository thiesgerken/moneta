-- The per-user view of the upstream data.
--
-- Rendering is a pure function of (expense, accounts), so it is computed here rather than
-- stored. That removes the delete-then-insert rerender step, which had no transaction around
-- it and could lose rendered rows if it died halfway.
--
-- The nested lists are aggregated into jsonb so the Rust structs keep their plain Vec fields.
-- Timestamps inside that jsonb carry the session's UTC offset rather than always being Z.
-- chrono reads any offset correctly, so this is only about keeping the wire representation
-- stable across machines; the pool pins the session to UTC for that reason.

-- Whose books an expense shows up in: the owners of every account its transactions touch,
-- plus the owners of those accounts' mirrors.
CREATE VIEW expense_users AS
SELECT t.expense_id, a.user_id
FROM expense_transactions t
JOIN accounts a ON a.id = t.account_id
UNION
SELECT t.expense_id, b.user_id
FROM expense_transactions t
JOIN accounts a ON a.id = t.account_id
JOIN accounts b ON b.id = a.sync_account_id;

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
    coalesce(tx.transactions, '[]'::jsonb) AS transactions,
    coalesce(tx.effective_amount, 0) AS effective_amount,

    e.created_at,
    e.updated_at
FROM expenses e
JOIN expense_users eu ON eu.expense_id = e.id
LEFT JOIN LATERAL (
    SELECT jsonb_agg(jsonb_build_object(
        'categoryId', ec.category_id,
        'weight', ec.weight
    )) AS categories
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
        sum(r.effective_amount) AS effective_amount
    FROM expense_transactions_rendered r
    WHERE r.expense_id = e.id AND r.user_id = eu.user_id
) tx ON true;

CREATE VIEW balances_rendered AS
SELECT
    b.id AS id,
    b.id AS upstream_id,
    a.user_id,
    b.date,
    a.id AS account_id,
    b.amount,
    b.comments,
    b.created_at,
    b.updated_at
FROM balances b
JOIN accounts a ON a.id = b.account_id
UNION ALL
SELECT
    b.id,
    b.id,
    m.user_id,
    b.date,
    m.id,
    b.amount * CASE WHEN a.sync_invert THEN -1 ELSE 1 END,
    b.comments,
    b.created_at,
    b.updated_at
FROM balances b
JOIN accounts a ON a.id = b.account_id
JOIN accounts m ON m.id = a.sync_account_id;
