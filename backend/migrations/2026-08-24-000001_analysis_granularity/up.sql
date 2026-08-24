-- Let the totals be read a year at a time as well as a month at a time.
--
-- A month is the grain the analysis was written for, and it is the wrong one for the question
-- "what did the last decade cost": a hundred and twenty bars, of which apex can label a dozen.
-- The grain is the only thing that has to change -- everything downstream already works on
-- whatever periods it is handed, including the balance samples at their boundaries.
--
-- One parameter rather than a second set of functions: the bucketing is a single `date_trunc`
-- on the value axis, and on the accrual axis it is the step of one `generate_series` and the
-- length of the window a period's share is measured against. `p_grain` is the argument to
-- `date_trunc`, so it is the same word in all three places.
--
-- It defaults to 'month', which is what the SQL tests pass where they are not about the grain.
--
-- `analysis_category_expenses` is deliberately not recreated and so keeps calling
-- `analysis_movements` with four arguments, taking that default. It groups the periods away
-- before it does anything else, so the grain cannot reach its answer -- but only because
-- `period_starts` snaps the range to whole periods first. On a range that stopped mid-year,
-- 'year' would pull in a whole year of an accrual period's weight where 'month' pulls in the
-- months inside the window, and the treemap would stop agreeing with the bars above it.
-- Anyone changing that snapping has to give this function the grain as well.

DROP FUNCTION analysis_movements(uuid, timestamptz, timestamptz, boolean);

-- Where each movement is filed, and how much of it lands there.
--
-- One function rather than the same CASE in three, and it is also where the range filter
-- lives: on the accrual axis a period reaching past the range contributes only the periods
-- inside it, so the filter cannot stay on the transaction.
--
-- A range is assumed to be whole periods -- which is what `period_starts` hands it. A range
-- ending mid-period keeps that period whole on the accrual side, where the value side would
-- have cut it off at the day.
CREATE FUNCTION analysis_movements(
    p_user_id uuid,
    p_from timestamptz,
    p_to timestamptz,
    p_accrued boolean,
    p_grain text DEFAULT 'month'
)
RETURNS TABLE (
    period timestamptz,
    expense_id uuid,
    account_id uuid,
    amount double precision
)
LANGUAGE sql
STABLE
AS $$
    SELECT
        date_trunc(p_grain, t.date),
        t.expense_id,
        t.account_id,
        t.calculated_amount
    FROM expense_transactions_rendered t
    JOIN expenses e ON e.id = t.expense_id
    JOIN accounts a ON a.id = t.account_id
    WHERE NOT p_accrued
      AND t.user_id = p_user_id
      -- only the reader's own accounts: a transaction on somebody else's account is visible
      -- in their copy of the expense but never moved their money
      AND a.user_id = p_user_id
      AND t.date >= p_from AND t.date < p_to
      AND t.calculated_amount <> 0
      -- a template is a stanza for a rule, not something that happened
      AND NOT e.is_template

    UNION ALL

    SELECT
        s.period,
        t.expense_id,
        t.account_id,
        t.calculated_amount * s.weight
    FROM expense_transactions_rendered t
    JOIN expenses e ON e.id = t.expense_id
    JOIN accounts a ON a.id = t.account_id
    CROSS JOIN LATERAL (
        SELECT
            gs AS period,
            -- days of the accrual period that fall in this one, over the days it has in
            -- total. Both ends inclusive: accrual_end is the last day the expense covers,
            -- and a period of one day is one day rather than none.
            (LEAST(e.accrual_end::date, (gs + ('1 ' || p_grain)::interval)::date - 1)
                - GREATEST(e.accrual_start::date, gs::date) + 1)::double precision
                / (e.accrual_end::date - e.accrual_start::date + 1) AS weight
        FROM generate_series(
            GREATEST(date_trunc(p_grain, e.accrual_start), date_trunc(p_grain, p_from)),
            -- p_to is exclusive, so the last period that can be filed starts before it
            LEAST(
                date_trunc(p_grain, e.accrual_end),
                date_trunc(p_grain, p_to - interval '1 microsecond')
            ),
            ('1 ' || p_grain)::interval
        ) gs
    ) s
    WHERE p_accrued
      AND t.user_id = p_user_id
      AND a.user_id = p_user_id
      AND t.calculated_amount <> 0
      AND NOT e.is_template
$$;

DROP FUNCTION analysis_month_totals(uuid, timestamptz, timestamptz, boolean);

CREATE FUNCTION analysis_period_totals(
    p_user_id uuid,
    p_from timestamptz,
    p_to timestamptz,
    p_accrued boolean DEFAULT false,
    p_grain text DEFAULT 'month'
)
RETURNS TABLE (
    period timestamptz,
    -- null on the period's roll-up row
    account_id uuid,
    income double precision,
    expenses double precision,
    expense_count bigint
)
LANGUAGE sql
STABLE
AS $$
    -- Once per (period, account) and once per period across them, which is what makes the
    -- roll-up net a transfer out while the accounts it moved between each keep their leg.
    -- Legs are netted within a period, not across one: a transfer that leaves in May and
    -- lands in June really was gone in between.
    WITH nets AS (
        SELECT m.period, m.account_id, m.expense_id, sum(m.amount) AS net
        FROM analysis_movements(p_user_id, p_from, p_to, p_accrued, p_grain) m
        GROUP BY GROUPING SETS ((m.period, m.account_id, m.expense_id), (m.period, m.expense_id))
    )
    SELECT
        n.period,
        n.account_id,
        coalesce(sum(n.net) FILTER (WHERE n.net > 0), 0),
        coalesce(sum(n.net) FILTER (WHERE n.net < 0), 0),
        -- an expense that cost nothing still happened, so the count does not ask about `net`.
        -- On the accrual axis one expense counts in every period it runs through, because it
        -- is one of the things that period cost.
        count(DISTINCT n.expense_id)
    FROM nets n
    GROUP BY 1, 2
$$;

DROP FUNCTION analysis_category_totals(uuid, timestamptz, timestamptz, boolean);

CREATE FUNCTION analysis_category_totals(
    p_user_id uuid,
    p_from timestamptz,
    p_to timestamptz,
    p_accrued boolean DEFAULT false,
    p_grain text DEFAULT 'month'
)
RETURNS TABLE (
    period timestamptz,
    category_id uuid,
    income double precision,
    expenses double precision
)
LANGUAGE sql
STABLE
AS $$
    WITH moved AS (
        SELECT m.period, m.expense_id, sum(m.amount) AS amount
        FROM analysis_movements(p_user_id, p_from, p_to, p_accrued, p_grain) m
        GROUP BY 1, 2
    ),
    weighted AS (
        SELECT
            m.period,
            ec.category_id,
            -- A weight is a share of the expense, not a multiplier on it -- see
            -- `normalise_category_weights`. An expense with no categorization has a null
            -- weight and so a null share; it still has to land somewhere, at full amount,
            -- under the null category.
            --
            -- The partition names the period as well as the expense: on the accrual axis
            -- `moved` holds one row per period an expense runs through, and a partition by
            -- expense alone would sum the weights once per period and shrink every share by
            -- that factor.
            m.amount * coalesce(
                ec.weight / sum(ec.weight) OVER (PARTITION BY m.period, m.expense_id),
                1
            ) AS amount
        FROM moved m
        LEFT JOIN expense_categorizations ec ON ec.expense_id = m.expense_id
    )
    SELECT
        w.period,
        w.category_id,
        coalesce(sum(w.amount) FILTER (WHERE w.amount > 0), 0),
        coalesce(sum(w.amount) FILTER (WHERE w.amount < 0), 0)
    FROM weighted w
    GROUP BY 1, 2
$$;
