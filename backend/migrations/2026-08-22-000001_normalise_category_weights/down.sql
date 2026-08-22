CREATE OR REPLACE FUNCTION analysis_category_totals(
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

CREATE OR REPLACE FUNCTION analysis_category_expenses(
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
