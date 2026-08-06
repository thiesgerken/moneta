-- Count what an expense cost, not what its legs did.
--
-- Income and spending were split per transaction: every positive amount was income, every
-- negative one spending. Moving 1.000 between two of your own accounts then reported 1.000
-- of income and 1.000 of spending in the same month, for money that never arrived or left.
-- A month of savings transfers read as large income and spending that never happened.
--
-- An expense now nets across the accounts it touches before it is filed, so what counts is
-- what it actually cost: a transfer nets to nothing, a transfer that cost 5 in fees is 5 of
-- spending, and money moved to an account you do not own is gone in full -- the leg on the
-- other side is not yours and was never in the sum.
--
-- The per-account rows net within their own account, so they still describe what flowed
-- through it: both halves of a transfer stay visible there, one on each side. They therefore
-- no longer add up to the month, which is the point of the roll-up row.
--
-- analysis_category_totals already worked this way, which is why a transfer never showed up
-- as a category.
DROP FUNCTION analysis_month_totals(uuid, timestamptz, timestamptz);

CREATE FUNCTION analysis_month_totals(
    p_user_id uuid,
    p_from timestamptz,
    p_to timestamptz
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
          -- only the reader's own accounts: a transaction on somebody else's account is
          -- visible in their copy of the expense but never moved their money
          AND a.user_id = p_user_id
          AND t.date >= p_from AND t.date < p_to
          AND t.calculated_amount <> 0
          -- a template is a stanza for a rule, not something that happened
          AND NOT e.is_template
    ),
    -- Once per (month, account) and once per month across them, which is what makes the
    -- roll-up net a transfer out while the accounts it moved between each keep their leg.
    -- Legs are netted within a month, not across one: a transfer that leaves in May and
    -- lands in June really was gone in between.
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
        -- an expense that cost nothing still happened, so the count does not ask about `net`
        count(DISTINCT n.expense_id)
    FROM nets n
    GROUP BY 1, 2
$$;
