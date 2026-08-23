-- Back to expanding every expense and letting the empty series do the filtering.
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
$$;
