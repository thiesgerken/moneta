DROP FUNCTION analysis_month_totals(uuid, timestamptz, timestamptz);

CREATE FUNCTION analysis_month_totals(
    p_user_id uuid,
    p_from timestamptz,
    p_to timestamptz
)
RETURNS TABLE (
    month timestamptz,
    account_id uuid,
    income double precision,
    expenses double precision,
    expense_count bigint
)
LANGUAGE sql
STABLE
AS $$
    SELECT
        date_trunc('month', t.date),
        t.account_id,
        coalesce(sum(t.calculated_amount) FILTER (WHERE t.calculated_amount > 0), 0),
        coalesce(sum(t.calculated_amount) FILTER (WHERE t.calculated_amount < 0), 0),
        count(DISTINCT t.expense_id)
    FROM expense_transactions_rendered t
    JOIN expenses e ON e.id = t.expense_id
    JOIN accounts a ON a.id = t.account_id
    WHERE t.user_id = p_user_id
      AND a.user_id = p_user_id
      AND t.date >= p_from AND t.date < p_to
      AND t.calculated_amount <> 0
      AND NOT e.is_template
    GROUP BY 1, 2
$$;
