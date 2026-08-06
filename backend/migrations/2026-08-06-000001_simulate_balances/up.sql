-- What an account was worth at a point in time, for as many points as you ask for.
--
-- This lives in the database rather than in a Rust string so that tests/sql can exercise it:
-- the rendered views have a test harness and the balance computation did not, even though it
-- is the part with the arithmetic in it.
--
-- Every column reference in the body is alias-qualified on purpose. The RETURNS TABLE names
-- are in scope inside the body, and `amount`, `account_id` and `date` all collide with
-- columns of the tables being read.
CREATE FUNCTION simulate_balances(
    p_user_id uuid,
    p_samples timestamptz[],
    p_account_id uuid DEFAULT NULL
)
RETURNS TABLE (
    sample_idx bigint,
    sample_at timestamptz,
    account_id uuid,
    balance_id uuid,
    amount double precision,
    expense_count bigint
)
LANGUAGE sql
STABLE
AS $$
    WITH samples AS (
        SELECT s.at, s.idx FROM unnest(p_samples) WITH ORDINALITY AS s(at, idx)
    ),
    acc AS (
        SELECT a.id FROM accounts a
        WHERE a.user_id = p_user_id AND (p_account_id IS NULL OR a.id = p_account_id)
    ),
    -- Rendering the transactions is the expensive half: `expense_users` is a UNION over the
    -- whole transaction table and cannot be narrowed to one account, so asking the view per
    -- (sample, account) rebuilds it once per sample. Materialize it once per call instead --
    -- a hundred samples then cost one rendering rather than a hundred.
    tx AS MATERIALIZED (
        SELECT t.account_id, t.date, t.expense_id, t.calculated_amount
        FROM expense_transactions_rendered t
        JOIN expenses e ON e.id = t.expense_id
        WHERE t.user_id = p_user_id AND t.calculated_amount <> 0
          -- A template is a stanza a rule copies, not something that happened -- but it
          -- carries real transactions with real dates, since `instantiate` shifts them onto
          -- the occurrence. Integrating them moves every balance after the template's own date.
          AND NOT e.is_template
    ),
    -- an account with no recorded balance at all has nothing to anchor to and drops out here
    anchors AS (
        SELECT s.idx, s.at, a.id AS account_id,
               n.id AS balance_id, n.date AS balance_date, n.amount
        FROM samples s
        CROSS JOIN acc a
        CROSS JOIN LATERAL (
            SELECT c.id, c.date, c.amount FROM (
                (SELECT b.id, b.date, b.amount FROM balances_rendered b
                  WHERE b.user_id = p_user_id AND b.account_id = a.id AND b.date <= s.at
                  ORDER BY b.date DESC LIMIT 1)
                UNION ALL
                -- integrating forward from the past alone would leave an account blank until
                -- its first recorded balance, which at the left edge of a long chart is most
                -- of them
                (SELECT b.id, b.date, b.amount FROM balances_rendered b
                  WHERE b.user_id = p_user_id AND b.account_id = a.id AND b.date > s.at
                  ORDER BY b.date ASC LIMIT 1)
            ) c
            -- an exact tie goes to the earlier balance, so the answer does not depend on row
            -- order
            ORDER BY abs(extract(epoch FROM (c.date - s.at))), c.date
            LIMIT 1
        ) n
    )
    SELECT
        an.idx,
        an.at,
        an.account_id,
        an.balance_id,
        -- a balance dated after the sample is integrated backwards to reach it
        an.amount::double precision
            + CASE WHEN an.balance_date <= an.at THEN 1 ELSE -1 END
              * coalesce(sum(t.calculated_amount), 0),
        -- of expenses, not transactions: an expense booking twice against the same account is
        -- one thing that happened
        count(DISTINCT t.expense_id)
    FROM anchors an
    -- one set-based join rather than an aggregate per anchor, which is what keeps this from
    -- scanning the transactions once per sample
    LEFT JOIN tx t
      ON t.account_id = an.account_id
      -- a balance is stated before the expenses of that same instant, so the interval is
      -- half-open at the top whichever direction it runs in
     AND t.date >= least(an.balance_date, an.at)
     AND t.date <  greatest(an.balance_date, an.at)
    GROUP BY an.idx, an.at, an.account_id, an.balance_id, an.balance_date, an.amount
$$;
