-- A categorization weight is a share of the expense, not a multiplier on it.
--
-- Nothing divided by the sum of the weights, so an expense in two categories at weight 1 each
-- landed in both at its full amount and the category breakdown summed to more than the month
-- actually spent -- while analysis_month_totals, which never looks at a categorization, kept
-- reporting the right figure underneath it.
--
-- A share is what everything else already assumed: the importer writes `w / total`, the
-- expense table's colour stripes divide by the sum, and the doc comment on CategoryTotal says
-- "weighted share". Normalising here rather than on the rows means no backfill and no
-- constraint to keep the stored weights summing to one -- 1/1 and 0.5/0.5 say the same thing.
--
-- The partition has to name the month as well as the expense in analysis_category_totals: on
-- the accrual axis `moved` holds one row per month an expense runs through, and a partition by
-- expense alone would sum the weights once per month and shrink every share by that factor.
-- analysis_category_expenses groups the months away first, so there the expense is enough.

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
            -- an expense with no categorization has a null weight and so a null share; it
            -- still has to land somewhere, at full amount, under the null category
            m.amount * coalesce(
                ec.weight / sum(ec.weight) OVER (PARTITION BY m.month, m.expense_id),
                1
            ) AS amount
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
            m.amount * coalesce(
                ec.weight / sum(ec.weight) OVER (PARTITION BY m.expense_id),
                1
            ) AS amount
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
