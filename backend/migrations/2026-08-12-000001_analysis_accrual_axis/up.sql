-- Let the totals be read on either date: when the money moved, or when the cost was incurred.
--
-- Every analysis figure so far has been filed under the transaction's own date. That answers
-- "what left my accounts in March", which is the right question for a statement and the wrong
-- one for a subscription paid once for six months: it lands entirely in the month it was paid
-- and the other five look cheaper than they were.
--
-- Switching the axis is not a different GROUP BY. An expense that accrues over six months
-- belongs to six of them, in proportion, so the movement has to be spread before it is filed.
-- The model is the linear consumption `analysis.prepayments` already reports a remainder from:
-- an equal share of the cost per day of the period.
--
-- What deliberately does NOT move: simulate_balances and analysis_statement. An accrued amount
-- is not money sitting in an account, and the *Fehler* column reconciles the totals against
-- recorded balances -- there is nothing to reconcile a sixth of a payment against.

-- Where each movement is filed, and how much of it lands there.
--
-- One function rather than the same CASE in three, and it is also where the range filter
-- lives: on the accrual axis a period reaching past the range contributes only the months
-- inside it, so the filter cannot stay on the transaction.
--
-- Months are the grain on both axes, so a range is assumed to be whole months -- which is what
-- `month_starts` hands it. A range ending mid-month keeps that month whole on the accrual
-- side, where the value side would have cut it off at the day.
CREATE FUNCTION analysis_movements(
    p_user_id uuid,
    p_from timestamptz,
    p_to timestamptz,
    p_accrued boolean
)
RETURNS TABLE (
    month timestamptz,
    expense_id uuid,
    account_id uuid,
    amount double precision
)
LANGUAGE sql
STABLE
AS $$
    SELECT
        date_trunc('month', t.date),
        t.expense_id,
        t.account_id,
        t.calculated_amount
    FROM expense_transactions_rendered t
    JOIN expenses e ON e.id = t.expense_id
    JOIN accounts a ON a.id = t.account_id
    WHERE NOT p_accrued
      AND t.user_id = p_user_id
      -- only the reader's own accounts: a transaction on somebody else's account is visible
      -- in their copy of the expense but never moved their money
      AND a.user_id = p_user_id
      AND t.date >= p_from AND t.date < p_to
      AND t.calculated_amount <> 0
      -- a template is a stanza for a rule, not something that happened
      AND NOT e.is_template

    UNION ALL

    SELECT
        s.month,
        t.expense_id,
        t.account_id,
        t.calculated_amount * s.weight
    FROM expense_transactions_rendered t
    JOIN expenses e ON e.id = t.expense_id
    JOIN accounts a ON a.id = t.account_id
    CROSS JOIN LATERAL (
        SELECT
            gs AS month,
            -- days of the period that fall in this month, over the days it has in total.
            -- Both ends inclusive: accrual_end is the last day the expense covers, and a
            -- period of one day is one day rather than none.
            (LEAST(e.accrual_end::date, (gs + interval '1 month')::date - 1)
                - GREATEST(e.accrual_start::date, gs::date) + 1)::double precision
                / (e.accrual_end::date - e.accrual_start::date + 1) AS weight
        FROM generate_series(
            GREATEST(date_trunc('month', e.accrual_start), date_trunc('month', p_from)),
            -- p_to is exclusive, so the last month that can be filed starts before it
            LEAST(date_trunc('month', e.accrual_end), date_trunc('month', p_to - interval '1 microsecond')),
            interval '1 month'
        ) gs
    ) s
    WHERE p_accrued
      AND t.user_id = p_user_id
      AND a.user_id = p_user_id
      AND t.calculated_amount <> 0
      AND NOT e.is_template
$$;

DROP FUNCTION analysis_month_totals(uuid, timestamptz, timestamptz);

CREATE FUNCTION analysis_month_totals(
    p_user_id uuid,
    p_from timestamptz,
    p_to timestamptz,
    p_accrued boolean DEFAULT false
)
RETURNS TABLE (
    month timestamptz,
    -- null on the month's roll-up row
    account_id uuid,
    income double precision,
    expenses double precision,
    expense_count bigint
)
LANGUAGE sql
STABLE
AS $$
    -- Once per (month, account) and once per month across them, which is what makes the
    -- roll-up net a transfer out while the accounts it moved between each keep their leg.
    -- Legs are netted within a month, not across one: a transfer that leaves in May and
    -- lands in June really was gone in between.
    WITH nets AS (
        SELECT m.month, m.account_id, m.expense_id, sum(m.amount) AS net
        FROM analysis_movements(p_user_id, p_from, p_to, p_accrued) m
        GROUP BY GROUPING SETS ((m.month, m.account_id, m.expense_id), (m.month, m.expense_id))
    )
    SELECT
        n.month,
        n.account_id,
        coalesce(sum(n.net) FILTER (WHERE n.net > 0), 0),
        coalesce(sum(n.net) FILTER (WHERE n.net < 0), 0),
        -- an expense that cost nothing still happened, so the count does not ask about `net`.
        -- On the accrual axis one expense counts in every month it runs through, because it
        -- is one of the things that month cost.
        count(DISTINCT n.expense_id)
    FROM nets n
    GROUP BY 1, 2
$$;

DROP FUNCTION analysis_category_totals(uuid, timestamptz, timestamptz);

CREATE FUNCTION analysis_category_totals(
    p_user_id uuid,
    p_from timestamptz,
    p_to timestamptz,
    p_accrued boolean DEFAULT false
)
RETURNS TABLE (
    month timestamptz,
    category_id uuid,
    income double precision,
    expenses double precision
)
LANGUAGE sql
STABLE
AS $$
    WITH moved AS (
        SELECT m.month, m.expense_id, sum(m.amount) AS amount
        FROM analysis_movements(p_user_id, p_from, p_to, p_accrued) m
        GROUP BY 1, 2
    ),
    weighted AS (
        SELECT
            m.month,
            ec.category_id,
            -- an expense with no categorization still has to land somewhere, at full weight
            m.amount * coalesce(ec.weight, 1) AS amount
        FROM moved m
        LEFT JOIN expense_categorizations ec ON ec.expense_id = m.expense_id
    )
    SELECT
        w.month,
        w.category_id,
        coalesce(sum(w.amount) FILTER (WHERE w.amount > 0), 0),
        coalesce(sum(w.amount) FILTER (WHERE w.amount < 0), 0)
    FROM weighted w
    GROUP BY 1, 2
$$;

DROP FUNCTION analysis_category_expenses(uuid, timestamptz, timestamptz, integer);

CREATE FUNCTION analysis_category_expenses(
    p_user_id uuid,
    p_from timestamptz,
    p_to timestamptz,
    p_limit integer,
    p_accrued boolean DEFAULT false
)
RETURNS TABLE (
    category_id uuid,
    expense_id uuid,
    title text,
    amount double precision
)
LANGUAGE sql
STABLE
AS $$
    WITH moved AS (
        SELECT m.expense_id, e.title, sum(m.amount) AS amount
        FROM analysis_movements(p_user_id, p_from, p_to, p_accrued) m
        JOIN expenses e ON e.id = m.expense_id
        GROUP BY 1, 2
    ),
    weighted AS (
        SELECT
            ec.category_id,
            m.expense_id,
            m.title,
            m.amount * coalesce(ec.weight, 1) AS amount
        FROM moved m
        LEFT JOIN expense_categorizations ec ON ec.expense_id = m.expense_id
    ),
    ranked AS (
        SELECT
            w.*,
            row_number() OVER (PARTITION BY w.category_id ORDER BY abs(w.amount) DESC) AS rn
        FROM weighted w
    )
    SELECT r.category_id, r.expense_id, r.title, r.amount
    FROM ranked r
    WHERE r.rn <= p_limit
$$;
