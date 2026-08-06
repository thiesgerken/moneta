-- Count a month's expenses once, not once per account they touch.
--
-- `expense_count` is per (month, account), and the caller summed those to get the month's
-- total. An expense booking against two of your own accounts -- a transfer, or a purchase
-- split across a card and a virtual counter-account -- was then counted twice. A month then
-- reported more expenses than the expense list showed.
--
-- A distinct count cannot be recovered by adding up per-account counts, so the function now
-- emits the roll-up itself: rows with a null account_id are the month across all accounts,
-- and their count is distinct over the whole month.
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
    -- the second set is the same rows grouped by month alone, which is what makes its
    -- count(DISTINCT ...) span the accounts rather than repeat within them
    GROUP BY GROUPING SETS (
        (date_trunc('month', t.date), t.account_id),
        (date_trunc('month', t.date))
    )
$$;
