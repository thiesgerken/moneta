-- An account starts empty, so there is always something to integrate from.
--
-- Anchoring only on recorded balances left an account with none out of every answer, and that
-- is a rule about bookkeeping rather than about money: an account nobody has ever written a
-- balance for still had nothing in it before its first transaction. moneydb enforced the same
-- rule from the other side by refusing to show an account until it had a balance, which is why
-- most of the imported ones open with a 0,00 EUR row dated the day that database was set up.
--
-- Offered at the beginning of time, the empty balance loses the nearest-anchor comparison to
-- every recorded one however far away, so it changes nothing for an account that has any --
-- including those whose first recorded balance is not zero, where money existed before
-- anyone started writing it down and integrating from zero would be wrong.
--
-- Both functions carry their own copy of the anchor lookup, for the reasons in
-- 2026-08-07-000002; they are replaced together.

DROP FUNCTION simulate_balances(uuid, timestamptz[], uuid);
DROP FUNCTION count_balance_expenses(uuid, timestamptz, uuid);

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
    -- null when the anchor is the empty balance every account starts from, which is no
    -- recorded balance and so has nothing to compare a simulated amount against
    balance_id uuid,
    amount double precision
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
        SELECT t.account_id, t.date, t.calculated_amount
        FROM expense_transactions_rendered t
        JOIN expenses e ON e.id = t.expense_id
        WHERE t.user_id = p_user_id AND t.calculated_amount <> 0
          -- A template is a stanza a rule copies, not something that happened -- but it
          -- carries real transactions with real dates, since `instantiate` shifts them onto
          -- the occurrence. Integrating them moves every balance after the template's own date.
          AND NOT e.is_template
    ),
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
                UNION ALL
                -- the account before it held anything, which is always the furthest away and
                -- therefore only ever wins when there is nothing else
                (SELECT NULL::uuid, '-infinity'::timestamptz, 0)
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
              * coalesce(sum(t.calculated_amount), 0)
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

-- How many expenses stand between each account's anchor balance and `p_at`.
--
-- One date rather than an array: this is what the balance table prints beside a reference
-- balance, and it is the aggregate that made sampling a whole range expensive. Anchoring is
-- the same as in `simulate_balances`, down to which balance wins a tie -- the count and the
-- amount describe the same interval or they describe nothing.
CREATE FUNCTION count_balance_expenses(
    p_user_id uuid,
    p_at timestamptz,
    p_account_id uuid DEFAULT NULL
)
RETURNS TABLE (
    account_id uuid,
    expense_count bigint
)
LANGUAGE sql
STABLE
AS $$
    WITH acc AS (
        SELECT a.id FROM accounts a
        WHERE a.user_id = p_user_id AND (p_account_id IS NULL OR a.id = p_account_id)
    ),
    tx AS MATERIALIZED (
        SELECT t.account_id, t.date, t.expense_id
        FROM expense_transactions_rendered t
        JOIN expenses e ON e.id = t.expense_id
        WHERE t.user_id = p_user_id AND t.calculated_amount <> 0
          AND NOT e.is_template
    ),
    anchors AS (
        SELECT a.id AS account_id, n.date AS balance_date
        FROM acc a
        CROSS JOIN LATERAL (
            SELECT c.date FROM (
                (SELECT b.date FROM balances_rendered b
                  WHERE b.user_id = p_user_id AND b.account_id = a.id AND b.date <= p_at
                  ORDER BY b.date DESC LIMIT 1)
                UNION ALL
                (SELECT b.date FROM balances_rendered b
                  WHERE b.user_id = p_user_id AND b.account_id = a.id AND b.date > p_at
                  ORDER BY b.date ASC LIMIT 1)
                UNION ALL
                (SELECT '-infinity'::timestamptz)
            ) c
            ORDER BY abs(extract(epoch FROM (c.date - p_at))), c.date
            LIMIT 1
        ) n
    )
    SELECT
        an.account_id,
        -- of expenses, not transactions: an expense booking twice against the same account is
        -- one thing that happened
        count(DISTINCT t.expense_id)
    FROM anchors an
    LEFT JOIN tx t
      ON t.account_id = an.account_id
     AND t.date >= least(an.balance_date, p_at)
     AND t.date <  greatest(an.balance_date, p_at)
    GROUP BY an.account_id
$$;
