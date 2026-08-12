DROP FUNCTION analysis_category_expenses(uuid, timestamptz, timestamptz, integer, boolean);
DROP FUNCTION analysis_category_totals(uuid, timestamptz, timestamptz, boolean);
DROP FUNCTION analysis_month_totals(uuid, timestamptz, timestamptz, boolean);
DROP FUNCTION analysis_movements(uuid, timestamptz, timestamptz, boolean);

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
    WITH tx AS (
        SELECT
            date_trunc('month', t.date) AS month,
            t.expense_id,
            t.account_id,
            t.calculated_amount AS amount
        FROM expense_transactions_rendered t
        JOIN expenses e ON e.id = t.expense_id
        JOIN accounts a ON a.id = t.account_id
        WHERE t.user_id = p_user_id
          AND a.user_id = p_user_id
          AND t.date >= p_from AND t.date < p_to
          AND t.calculated_amount <> 0
          AND NOT e.is_template
    ),
    nets AS (
        SELECT month, account_id, expense_id, sum(amount) AS net
        FROM tx
        GROUP BY GROUPING SETS ((month, account_id, expense_id), (month, expense_id))
    )
    SELECT
        n.month,
        n.account_id,
        coalesce(sum(n.net) FILTER (WHERE n.net > 0), 0),
        coalesce(sum(n.net) FILTER (WHERE n.net < 0), 0),
        count(DISTINCT n.expense_id)
    FROM nets n
    GROUP BY 1, 2
$$;

CREATE FUNCTION analysis_category_totals(
    p_user_id uuid,
    p_from timestamptz,
    p_to timestamptz
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
        SELECT
            date_trunc('month', t.date) AS month,
            t.expense_id,
            sum(t.calculated_amount) AS amount
        FROM expense_transactions_rendered t
        JOIN expenses e ON e.id = t.expense_id
        JOIN accounts a ON a.id = t.account_id
        WHERE t.user_id = p_user_id
          AND a.user_id = p_user_id
          AND t.date >= p_from AND t.date < p_to
          AND t.calculated_amount <> 0
          AND NOT e.is_template
        GROUP BY 1, 2
    ),
    weighted AS (
        SELECT m.month, ec.category_id, m.amount * coalesce(ec.weight, 1) AS amount
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

CREATE FUNCTION analysis_category_expenses(
    p_user_id uuid,
    p_from timestamptz,
    p_to timestamptz,
    p_limit integer
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
        SELECT t.expense_id, e.title, sum(t.calculated_amount) AS amount
        FROM expense_transactions_rendered t
        JOIN expenses e ON e.id = t.expense_id
        JOIN accounts a ON a.id = t.account_id
        WHERE t.user_id = p_user_id
          AND a.user_id = p_user_id
          AND t.date >= p_from AND t.date < p_to
          AND t.calculated_amount <> 0
          AND NOT e.is_template
        GROUP BY 1, 2
    ),
    weighted AS (
        SELECT ec.category_id, m.expense_id, m.title, m.amount * coalesce(ec.weight, 1) AS amount
        FROM moved m
        LEFT JOIN expense_categorizations ec ON ec.expense_id = m.expense_id
    ),
    ranked AS (
        SELECT w.*, row_number() OVER (PARTITION BY w.category_id ORDER BY abs(w.amount) DESC) AS rn
        FROM weighted w
    )
    SELECT r.category_id, r.expense_id, r.title, r.amount
    FROM ranked r
    WHERE r.rn <= p_limit
$$;
