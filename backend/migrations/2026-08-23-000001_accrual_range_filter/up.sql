-- Stop the accrual axis from expanding every expense that ever existed.
--
-- The accrued branch of `analysis_movements` had no range filter at all. It reached the right
-- answer anyway -- an expense whose period lies outside the range gets an empty
-- `generate_series` and contributes nothing -- but it got there by rendering every transaction
-- the user has and calling the lateral on each. Filtering before the expansion rather than
-- through it is the same set of rows, computed from a fraction of the input.
--
-- The bigger cost was one the plan does not show as work. Postgres assumes any
-- `generate_series` returns 1000 rows; here the real average is 0.12, so the estimate came out
-- ~8000x too high and pushed the query past `jit_inline_above_cost` and
-- `jit_optimize_above_cost`. LLVM then spent over a second optimising a query that runs in
-- 120 ms. Cutting the input cuts the estimate with it.
--
-- The predicate is written on `date_trunc` rather than on the raw dates because that is
-- exactly when the `generate_series` above is non-empty, for any range. Comparing the raw
-- dates would be equivalent only for ranges that are whole months, which is all `month_starts`
-- ever passes but not a promise the function itself makes.
CREATE OR REPLACE FUNCTION analysis_movements(
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
      AND date_trunc('month', e.accrual_end) >= date_trunc('month', p_from)
      AND date_trunc('month', e.accrual_start) <= date_trunc('month', p_to - interval '1 microsecond')
$$;
