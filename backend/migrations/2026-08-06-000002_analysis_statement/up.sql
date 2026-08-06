-- One account's ledger over a range: the computed opening balance, every expense touching the
-- account, and the balances actually recorded for it, in one chronological sequence.
--
-- This is a reconciliation view, not the expense list with a filter. Recorded balances
-- re-anchor the running total, and where one disagrees with the total carried into it the gap
-- is reported as `drift` -- that gap is the signal that an expense is missing.
--
-- In the database rather than a Rust string for the same reasons as simulate_balances:
-- tests/sql can reach it, and the ordering rule below is exactly the kind that goes wrong
-- quietly when two clients each reimplement it.
CREATE FUNCTION analysis_statement(
    p_user_id uuid,
    p_account_id uuid,
    p_from timestamptz,
    p_to timestamptz
)
RETURNS TABLE (
    seq bigint,
    kind text,
    entry_date timestamptz,
    title text,
    store text,
    expense_id uuid,
    amount double precision,
    balance double precision,
    drift double precision
)
LANGUAGE sql
STABLE
AS $$
    WITH entries AS (
        -- kind_order breaks ties within one instant: a balance is stated *before* the
        -- expenses of that same instant (balances_rendered.date includes expenses with
        -- date < it), and the opening balance is stated before everything.
        SELECT
            0 AS kind_order,
            'opening'::text AS kind,
            p_from AS entry_date,
            ''::text AS title,
            ''::text AS store,
            NULL::uuid AS expense_id,
            NULL::double precision AS amount,
            -- an account with no recorded balance anywhere cannot be anchored; starting from
            -- zero at least makes the movements in the range readable
            coalesce(
                (SELECT s.amount FROM simulate_balances(p_user_id, ARRAY[p_from], p_account_id) s),
                0
            ) AS anchor

        UNION ALL

        SELECT
            1, 'balance', b.date, ''::text, ''::text, NULL::uuid, NULL::double precision,
            b.amount::double precision
        FROM balances_rendered b
        WHERE b.user_id = p_user_id AND b.account_id = p_account_id
          AND b.date >= p_from AND b.date <= p_to

        UNION ALL

        -- grouped by (expense, instant): splitting one purchase into two legs on the same
        -- account at the same moment is an accounting detail, not two movements. Legs on
        -- different dates stay separate rows, which is what a bank statement shows.
        SELECT
            2, 'expense', t.date, e.title, e.store, t.expense_id,
            sum(t.calculated_amount), NULL::double precision
        FROM expense_transactions_rendered t
        JOIN expenses e ON e.id = t.expense_id
        WHERE t.user_id = p_user_id AND t.account_id = p_account_id
          AND t.date >= p_from AND t.date <= p_to
          -- a rule's stanza carries a real account and a real date but never happened, and
          -- the opening balance already leaves it out
          AND NOT e.is_template
        GROUP BY t.date, e.title, e.store, t.expense_id
        HAVING sum(t.calculated_amount) <> 0
    ),
    -- Every anchor row (the opening, and each recorded balance) starts an island. Within one
    -- island the balance is the anchor plus the expenses since it, which is what makes a
    -- recorded balance override the total carried into it instead of being absorbed by it.
    islands AS (
        SELECT
            e.*,
            count(e.anchor) OVER (
                ORDER BY e.entry_date, e.kind_order ROWS UNBOUNDED PRECEDING
            ) AS island
        FROM entries e
    ),
    running AS (
        SELECT
            i.*,
            max(i.anchor) OVER (PARTITION BY i.island)
                + sum(coalesce(i.amount, 0)) OVER (
                    PARTITION BY i.island
                    ORDER BY i.entry_date, i.kind_order ROWS UNBOUNDED PRECEDING
                ) AS balance,
            row_number() OVER (ORDER BY i.entry_date, i.kind_order) AS seq
        FROM islands i
    )
    SELECT
        r.seq,
        r.kind,
        r.entry_date,
        r.title,
        r.store,
        r.expense_id,
        r.amount,
        r.balance,
        -- what the recorded balance says minus what was carried into it
        CASE WHEN r.kind = 'balance'
             THEN r.anchor - lag(r.balance) OVER (ORDER BY r.entry_date, r.kind_order)
        END AS drift
    FROM running r
    -- accumulated ascending, returned descending: newest first is what you want to read
    ORDER BY r.entry_date DESC, r.kind_order DESC
$$;
