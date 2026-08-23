-- The wealth curve read on the accrual axis, as a correction to the one read on value dates.
--
-- `simulate_balances` still does not move, and neither does anything per account: an account's
-- balance is money that sat in it, reconciled against readings somebody took, and a sixth of a
-- payment is not something a bank ever held. Over all accounts together there is nothing left
-- to reconcile against, and the question becomes answerable -- what were you worth, counting
-- what you had already paid for but not yet used, and what you had already used but not yet
-- paid for.
--
-- Both directions fall out of one term. A transaction contributes to wealth at `t`
--
--     on the value axis:   amount, once its own date has passed
--     on the accrual axis: amount * consumed(t), whenever the money moved
--
-- so the correction to add to the value-axis total is the difference of the two:
--
--     amount * (consumed(t) - [date < t])
--
-- A year paid in January is bought but unconsumed, and the correction hands the unconsumed
-- part back as wealth until it runs out. A bill for the first half-year that arrives in July
-- is consumed but unpaid, and the correction takes it away before the money leaves. Outside
-- every open period the two terms agree and the correction is exactly zero, which is the
-- property `tests/sql/accrual_wealth.sql` leans on.
--
-- `consumed` is the continuous form from concepts.md, the one `analysis.prepayments` reports a
-- remainder from, rather than the whole-day weights of `analysis_movements`: this is sampled
-- at instants and asked what is left at one of them, not aggregated into month buckets.
CREATE FUNCTION accrual_adjustment(
    p_user_id uuid,
    p_samples timestamptz[]
)
RETURNS TABLE (
    sample_idx bigint,
    amount double precision
)
LANGUAGE sql
STABLE
AS $$
    WITH samples AS (
        SELECT s.at, s.idx FROM unnest(p_samples) WITH ORDINALITY AS s(at, idx)
    ),
    bounds AS (
        SELECT min(s.at) AS lo, max(s.at) AS hi FROM samples s
    ),
    -- The population has to be exactly the one `simulate_balances` integrates -- same user,
    -- same accounts, same template rule, same zero-amount rule. The correction is a difference
    -- between two readings of these rows, so a row present in one and missing from the other
    -- is a curve that never comes back together with the one it is drawn against.
    tx AS MATERIALIZED (
        SELECT t.date, t.calculated_amount, e.accrual_start, e.accrual_end
        FROM expense_transactions_rendered t
        JOIN expenses e ON e.id = t.expense_id
        JOIN accounts a ON a.id = t.account_id
        CROSS JOIN bounds b
        WHERE t.user_id = p_user_id
          -- only the reader's own accounts, which is what `simulate_balances` reaches by
          -- joining its anchors against them
          AND a.user_id = p_user_id
          AND t.calculated_amount <> 0
          AND NOT e.is_template
          -- An expense whose value was consumed on the day it was paid contributes `1 - 1` or
          -- `0 - 0` at every sample. Four out of five rows are that, and dropping them here
          -- rather than multiplying them by every sample is what keeps the join below to the
          -- handful of rows that can actually say something.
          AND NOT (e.accrual_start = e.accrual_end AND e.accrual_start = t.date)
          -- Likewise for a row whose period and payment both lie to one side of every sample:
          -- both terms are 0 below all of them and 1 above all of them. Written as an OR pair
          -- rather than as `greatest`/`least` so each half can still be an index scan.
          AND (e.accrual_end >= b.lo OR t.date >= b.lo)
          AND (e.accrual_start < b.hi OR t.date < b.hi)
    )
    SELECT
        s.idx,
        coalesce(sum(t.calculated_amount * (
            CASE
                -- A degenerate period is consumed the moment it starts. Strictly after, so
                -- that it cancels the value term exactly when the two dates coincide -- a
                -- balance is stated before the movements of its own instant, and this side of
                -- the subtraction has to keep the same convention as the other.
                WHEN t.accrual_end <= t.accrual_start
                    THEN CASE WHEN s.at > t.accrual_start THEN 1 ELSE 0 END
                ELSE least(1, greatest(0,
                    extract(epoch FROM (s.at - t.accrual_start))
                        / extract(epoch FROM (t.accrual_end - t.accrual_start))))
            END
            - CASE WHEN t.date < s.at THEN 1 ELSE 0 END
        )), 0)
    FROM samples s
    -- every sample against every row that survived the filter; there is no key to join on,
    -- since what a row contributes depends on where the sample falls inside its period
    LEFT JOIN tx t ON true
    GROUP BY s.idx
$$;
