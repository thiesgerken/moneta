-- Covers the accrual axis of analysis_movements and the three totals built on it: a cost
-- spread over the months it was for rather than filed under the day it was paid, the range
-- filter moving from the transaction to the period, and no money invented or lost either way.
BEGIN;

INSERT INTO users (id, name, full_name, password_hash) VALUES
    ('00000000-0000-7000-8000-000000000001', 'mine', 'Mine', 'x'),
    -- the periods that cross an edge of the range get their own user rather than joining the
    -- fixtures above, so the arithmetic the other checks pin down stays where it is
    ('00000000-0000-7000-8000-000000000002', 'edge', 'Edge', 'x');

INSERT INTO accounts (id, user_id, name, description, kind, availability, risk, opened, ownership_fraction) VALUES
    ('00000000-0000-7000-8000-0000000000a1', '00000000-0000-7000-8000-000000000001', 'a1', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0),
    ('00000000-0000-7000-8000-0000000000a2', '00000000-0000-7000-8000-000000000002', 'a2', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0);

INSERT INTO categories (id, name, description) VALUES
    ('00000000-0000-7000-8000-0000000000c1', 'Abo', ''),
    ('00000000-0000-7000-8000-0000000000c2', 'Haushalt', '');

INSERT INTO expenses (id, title, store, description, comments, reference_amount, accrual_start, accrual_end, is_template) VALUES
    -- paid once in June for June, July and August: 92 days, so 30/31/31 of -9200 is
    -- -3000/-3100/-3100. Chosen to divide exactly so a wrong denominator is visible.
    ('00000000-0000-7000-8000-0000000000f1', 'quartal', '', '', '', NULL, '2025-06-01Z', '2025-08-31Z', false),
    -- an ordinary expense: one day, so both axes agree about it
    ('00000000-0000-7000-8000-0000000000f2', 'punkt',   '', '', '', NULL, '2025-07-10Z', '2025-07-10Z', false),
    -- paid in June for September. On the value axis it is June's; on the accrual axis it is
    -- September's, which a range ending at September does not reach.
    ('00000000-0000-7000-8000-0000000000f3', 'voraus',  '', '', '', NULL, '2025-09-01Z', '2025-09-30Z', false),
    -- The two that decide which way round the range filter is written. Each runs across one
    -- edge of the range, so each contributes some of its months and not others: a filter that
    -- demanded the whole period sit inside the range would drop both, and every check above
    -- would still pass. 91 and 92 days at a round 100 cents a day, so a wrong denominator or
    -- a wrong month shows up as a whole number rather than as rounding.
    ('00000000-0000-7000-8000-0000000000f4', 'vorlauf', '', '', '', NULL, '2025-04-01Z', '2025-06-30Z', false),
    ('00000000-0000-7000-8000-0000000000f5', 'nachlauf','', '', '', NULL, '2025-08-01Z', '2025-10-31Z', false);

-- Two categories at weight 1 each on the expense that spans three months: the weights have
-- to be summed within a month, since the quarter is one row per month it runs through and
-- summing across all three would leave each category a sixth of the month instead of a half.
INSERT INTO expense_categorizations (expense_id, category_id, weight) VALUES
    ('00000000-0000-7000-8000-0000000000f1', '00000000-0000-7000-8000-0000000000c1', 1.0),
    ('00000000-0000-7000-8000-0000000000f1', '00000000-0000-7000-8000-0000000000c2', 1.0);

INSERT INTO expense_transactions (expense_id, account_id, idx, date, amount, fraction, comments, statement) VALUES
    ('00000000-0000-7000-8000-0000000000f1', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-15Z', -9200, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000f2', '00000000-0000-7000-8000-0000000000a1', 0, '2025-07-10Z',  -500, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000f3', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-20Z',  -700, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000f4', '00000000-0000-7000-8000-0000000000a2', 0, '2025-04-05Z', -9100, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000f5', '00000000-0000-7000-8000-0000000000a2', 0, '2025-08-15Z', -9200, NULL, '', '');

DO $$
DECLARE
    r record;
    failures int := 0;
    mine uuid := '00000000-0000-7000-8000-000000000001';
    edge uuid := '00000000-0000-7000-8000-000000000002';
    c1 uuid := '00000000-0000-7000-8000-0000000000c1';
    c2 uuid := '00000000-0000-7000-8000-0000000000c2';
    f1 uuid := '00000000-0000-7000-8000-0000000000f1';
    jun timestamptz := '2025-06-01Z';
    jul timestamptz := '2025-07-01Z';
    aug timestamptz := '2025-08-01Z';
    sep timestamptz := '2025-09-01Z';
    -- wide enough to hold every period and every payment, so the two axes have to agree
    -- about the total even though they disagree about every month of it
    y_from timestamptz := '2025-01-01Z';
    y_to timestamptz := '2026-01-01Z';
BEGIN
    FOR r IN
        WITH v AS (SELECT * FROM analysis_period_totals(mine, jun, sep, false)),
             a AS (SELECT * FROM analysis_period_totals(mine, jun, sep, true)),
             ca AS (SELECT * FROM analysis_category_totals(mine, jun, sep, true)),
             xa AS (SELECT * FROM analysis_category_expenses(mine, jun, sep, 5, true)),
             ea AS (SELECT * FROM analysis_period_totals(edge, jun, sep, true)),
             yv AS (SELECT * FROM analysis_period_totals(mine, y_from, y_to, false)),
             ya AS (SELECT * FROM analysis_period_totals(mine, y_from, y_to, true)),
        checks(label, actual, expected) AS (
            -- the value axis is unchanged: the quarterly payment and the prepayment both land
            -- in the month the money left, and August holds nothing at all
            SELECT 'value axis files june by payment',
                (SELECT round(expenses::numeric, 2) FROM v WHERE period = jun AND account_id IS NULL), -9900.00
            UNION ALL SELECT 'value axis july',
                (SELECT round(expenses::numeric, 2) FROM v WHERE period = jul AND account_id IS NULL), -500.00
            UNION ALL SELECT 'value axis has no august',
                (SELECT count(*) FROM v WHERE period = aug), 0

            -- the accrual axis spreads the quarter by days, and September's prepayment is out
            -- of a range that stops at September
            UNION ALL SELECT 'accrual axis june is 30 days of the quarter',
                (SELECT round(expenses::numeric, 2) FROM a WHERE period = jun AND account_id IS NULL), -3000.00
            UNION ALL SELECT 'accrual axis july adds the one-day expense',
                (SELECT round(expenses::numeric, 2) FROM a WHERE period = jul AND account_id IS NULL), -3600.00
            UNION ALL SELECT 'accrual axis reaches august, which the payment never did',
                (SELECT round(expenses::numeric, 2) FROM a WHERE period = aug AND account_id IS NULL), -3100.00
            UNION ALL SELECT 'a period outside the range contributes nothing',
                (SELECT count(*) FROM a WHERE round(expenses::numeric, 2) = -700.00), 0

            -- A period that starts before the range or ends after it still owns the months of
            -- it that are inside: April and May of the one, September and October of the
            -- other, are outside and stay outside. Both directions, because a filter written
            -- the wrong way round is symmetric and one of these alone would not see it.
            UNION ALL SELECT 'a period reaching in from before keeps its june',
                (SELECT round(expenses::numeric, 2) FROM ea WHERE period = jun AND account_id IS NULL), -3000.00
            UNION ALL SELECT 'a period reaching out past the end keeps its august',
                (SELECT round(expenses::numeric, 2) FROM ea WHERE period = aug AND account_id IS NULL), -3100.00
            -- and nothing leaks into the month between them, which neither period covers
            UNION ALL SELECT 'july lies between the two and stays empty',
                (SELECT count(*) FROM ea WHERE period = jul), 0

            -- an expense counts in every month it runs through, because it is one of the
            -- things that month cost
            UNION ALL SELECT 'the quarter counts in each of its months',
                (SELECT count(*) FROM a WHERE account_id IS NULL AND expense_count >= 1), 3
            UNION ALL SELECT 'july holds two of them',
                (SELECT expense_count FROM a WHERE period = jul AND account_id IS NULL), 2

            -- the per-account rows are spread the same way as the roll-up
            UNION ALL SELECT 'the account row is spread too',
                (SELECT round(expenses::numeric, 2) FROM a WHERE period = aug AND account_id IS NOT NULL), -3100.00

            -- categories ride on the same movements, and each of the two holds half of the
            -- month's share of the quarter rather than all of it
            UNION ALL SELECT 'the category gets half of june share only',
                (SELECT round(expenses::numeric, 2) FROM ca WHERE period = jun AND category_id = c1), -1500.00
            UNION ALL SELECT 'the other category gets the other half',
                (SELECT round(expenses::numeric, 2) FROM ca WHERE period = jun AND category_id = c2), -1500.00
            UNION ALL SELECT 'and half the quarter across the range',
                (SELECT round(sum(expenses)::numeric, 2) FROM ca WHERE category_id = c1), -4600.00
            -- weights summed across the months rather than within one would leave each
            -- category a sixth of the month, and reading them as multipliers would give it
            -- the whole month twice over. Both miss the month itself.
            UNION ALL SELECT 'the categories of a month sum to that month',
                (SELECT round(sum(expenses)::numeric, 2) FROM ca WHERE period = jun),
                (SELECT round(expenses::numeric, 2) FROM a WHERE period = jun AND account_id IS NULL)

            -- the treemap sums an expense's in-range portions back up
            UNION ALL SELECT 'the treemap shows the part inside the range',
                (SELECT round(sum(amount)::numeric, 2) FROM xa WHERE expense_id = f1), -9200.00
            UNION ALL SELECT 'split between the two categories it is filed under',
                (SELECT count(*) FROM xa WHERE expense_id = f1
                   AND round(amount::numeric, 2) = -4600.00), 2
            UNION ALL SELECT 'and leaves out the one whose period is outside it',
                (SELECT count(*) FROM xa WHERE expense_id = '00000000-0000-7000-8000-0000000000f3'), 0

            -- over a range that holds everything, the two axes disagree about every month and
            -- agree about the total: spreading moves money between months, it does not make
            -- or destroy any
            UNION ALL SELECT 'both axes come to the same total',
                (SELECT round(sum(income + expenses)::numeric, 2) FROM ya WHERE account_id IS NULL),
                (SELECT round(sum(income + expenses)::numeric, 2) FROM yv WHERE account_id IS NULL)
            UNION ALL SELECT 'which is what was actually paid',
                (SELECT round(sum(income + expenses)::numeric, 2) FROM ya WHERE account_id IS NULL), -10400.00
            -- and they really are different months, or the check above proves nothing
            UNION ALL SELECT 'the axes disagree about june',
                (SELECT count(*) FROM ya y JOIN yv w USING (period)
                 WHERE y.account_id IS NULL AND w.account_id IS NULL
                   AND y.period = jun AND round(y.expenses::numeric, 2) = round(w.expenses::numeric, 2)), 0
        )
        SELECT * FROM checks WHERE actual IS DISTINCT FROM expected
    LOOP
        RAISE WARNING 'FAIL % : got % want %', r.label, r.actual, r.expected;
        failures := failures + 1;
    END LOOP;

    IF failures > 0 THEN
        RAISE EXCEPTION '% accrual axis check(s) failed', failures;
    END IF;
    RAISE NOTICE 'all accrual axis checks passed';
END $$;

ROLLBACK;
