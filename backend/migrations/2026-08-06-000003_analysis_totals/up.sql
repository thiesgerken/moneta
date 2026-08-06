-- Income and spending per month, split by account and by category.
--
-- Both read expense_transactions_rendered exactly once and group, rather than being asked per
-- month: the view cannot be narrowed to a month cheaply (see expense_users), so one pass with
-- a GROUP BY is the difference between milliseconds and seconds.
--
-- The amounts are `calculated_amount`, the movement on the account itself, not
-- `effective_amount`. That is deliberate: the totals have to reconcile with the balances
-- simulate_balances reports, and those are account balances rather than ownership-weighted
-- shares of them. Mixing the two would make the integration error meaningless.

-- Per (month, account). `expense_count` counts expenses, not transactions, so an expense
-- booking twice against one account in one month is one thing that happened.
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
      -- only the reader's own accounts: a transaction on somebody else's account is visible
      -- in their copy of the expense but never moved their money
      AND a.user_id = p_user_id
      AND t.date >= p_from AND t.date < p_to
      AND t.calculated_amount <> 0
      -- a template is a stanza for a rule, not something that happened
      AND NOT e.is_template
    GROUP BY 1, 2
$$;

-- Per (month, category). An expense split across categories contributes its weighted share to
-- each, which is what `expense_categorizations.weight` is for; an uncategorized expense comes
-- back with a null category rather than being dropped, so the parts still sum to the whole.
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

-- The biggest expenses within each category over the whole range, which is the inner level of
-- the treemap. Ranked by absolute amount, so the same call serves the income and the spending
-- view; what a category's total does not account for becomes a single "rest" slice, computed
-- by the caller from the totals above rather than by a second pass here.
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
