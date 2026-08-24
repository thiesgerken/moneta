-- Covers the year grain: the same movements bucketed a year at a time instead of a month.
--
-- Two things can go wrong and only one of them is obvious. The obvious one is the bucket --
-- `date_trunc` taking the wrong word. The other is the window a spread cost is measured
-- against: on the accrual axis a period's share is the days of it that fall inside the
-- bucket, so the step of the series and the length of the bucket have to be the same unit.
-- Leave one of them at a month and a cost crossing New Year is measured against January.

BEGIN;

INSERT INTO users (id, name, full_name, password_hash) VALUES
    ('00000000-0000-7000-8000-0000000000e1', 'grain', 'Grain', 'x');

INSERT INTO accounts (id, user_id, name, description, kind, availability, risk, opened, ownership_fraction) VALUES
    ('00000000-0000-7000-8000-0000000000e2', '00000000-0000-7000-8000-0000000000e1', 'giro', '', 'debit', 'immediately', 'none', '2023-01-01Z', 1.0);

INSERT INTO categories (id, name, description) VALUES
    ('00000000-0000-7000-8000-0000000000e3', 'Abo', '');

INSERT INTO expenses (id, title, store, description, comments, reference_amount, accrual_start, accrual_end, is_template) VALUES
    -- three ordinary days, one of them in the following year
    ('00000000-0000-7000-8000-000000000f01', 'maerz', '', '', '', NULL, '2024-03-15Z', '2024-03-15Z', false),
    ('00000000-0000-7000-8000-000000000f02', 'november', '', '', '', NULL, '2024-11-10Z', '2024-11-10Z', false),
    ('00000000-0000-7000-8000-000000000f03', 'mai', '', '', '', NULL, '2025-05-20Z', '2025-05-20Z', false),
    -- paid once in December 2024 for the year that follows it: 365 days at a round 100 cents
    -- a day, so 2024 owns 31 of them and 2025 the other 334. The one expense that makes a
    -- year different from twelve months.
    ('00000000-0000-7000-8000-000000000f04', 'jahresabo', '', '', '', NULL, '2024-12-01Z', '2025-11-30Z', false),
    ('00000000-0000-7000-8000-000000000f05', 'gehalt', '', '', '', NULL, '2025-07-01Z', '2025-07-01Z', false);

INSERT INTO expense_categorizations (expense_id, category_id, weight) VALUES
    ('00000000-0000-7000-8000-000000000f04', '00000000-0000-7000-8000-0000000000e3', 1.0);

INSERT INTO expense_transactions (expense_id, account_id, idx, date, amount, fraction, comments, statement) VALUES
    ('00000000-0000-7000-8000-000000000f01', '00000000-0000-7000-8000-0000000000e2', 0, '2024-03-15Z',  -1000, NULL, '', ''),
    ('00000000-0000-7000-8000-000000000f02', '00000000-0000-7000-8000-0000000000e2', 0, '2024-11-10Z',  -2000, NULL, '', ''),
    ('00000000-0000-7000-8000-000000000f03', '00000000-0000-7000-8000-0000000000e2', 0, '2025-05-20Z',  -4000, NULL, '', ''),
    ('00000000-0000-7000-8000-000000000f04', '00000000-0000-7000-8000-0000000000e2', 0, '2024-12-01Z', -36500, NULL, '', ''),
    ('00000000-0000-7000-8000-000000000f05', '00000000-0000-7000-8000-0000000000e2', 0, '2025-07-01Z',  50000, NULL, '', '');

DO $$
DECLARE
    r record;
    failures int := 0;
    mine uuid := '00000000-0000-7000-8000-0000000000e1';
    abo uuid := '00000000-0000-7000-8000-0000000000e3';
    y24 timestamptz := '2024-01-01Z';
    y25 timestamptz := '2025-01-01Z';
    y26 timestamptz := '2026-01-01Z';
    mar timestamptz := '2024-03-01Z';
BEGIN
    FOR r IN
        WITH yv AS (SELECT * FROM analysis_period_totals(mine, y24, y26, false, 'year')),
             ya AS (SELECT * FROM analysis_period_totals(mine, y24, y26, true,  'year')),
             mv AS (SELECT * FROM analysis_period_totals(mine, y24, y26, false, 'month')),
             ma AS (SELECT * FROM analysis_period_totals(mine, y24, y26, true,  'month')),
             cy AS (SELECT * FROM analysis_category_totals(mine, y24, y26, true, 'year')),
        checks(label, actual, expected) AS (
            -- two years of movement are two rows, however many months they took
            SELECT 'a two year range rolls up twice',
                (SELECT count(*) FROM yv WHERE account_id IS NULL), 2
            UNION ALL SELECT 'and every bucket opens on a january',
                (SELECT count(*) FROM yv WHERE period <> date_trunc('year', period)), 0
            -- the month it happened in is a bucket of its own only at the finer grain
            UNION ALL SELECT 'march is a bucket by month',
                (SELECT count(*) FROM mv WHERE period = mar AND account_id IS NULL), 1
            UNION ALL SELECT 'and is not one by year',
                (SELECT count(*) FROM yv WHERE period = mar), 0

            -- the value axis files the whole subscription where it was paid
            UNION ALL SELECT 'value axis 2024 holds the payment in full',
                (SELECT round(expenses::numeric, 2) FROM yv WHERE period = y24 AND account_id IS NULL), -39500.00
            UNION ALL SELECT 'value axis 2025 holds only what was paid in it',
                (SELECT round(expenses::numeric, 2) FROM yv WHERE period = y25 AND account_id IS NULL), -4000.00

            -- The accrual axis splits it 31/334 across New Year. A window still a month long
            -- would measure December against January and land nowhere near this.
            UNION ALL SELECT 'accrual axis 2024 keeps its december of the subscription',
                (SELECT round(expenses::numeric, 2) FROM ya WHERE period = y24 AND account_id IS NULL), -6100.00
            UNION ALL SELECT 'accrual axis 2025 carries the rest of it',
                (SELECT round(expenses::numeric, 2) FROM ya WHERE period = y25 AND account_id IS NULL), -37400.00
            UNION ALL SELECT 'income is not spread into the year it was not earned',
                (SELECT round(income::numeric, 2) FROM ya WHERE period = y24 AND account_id IS NULL), 0.00

            -- the grain moves money between buckets and neither invents nor loses any of it
            UNION ALL SELECT 'the years sum to what the months do, on the value axis',
                (SELECT round(sum(income + expenses)::numeric, 2) FROM yv WHERE account_id IS NULL),
                (SELECT round(sum(income + expenses)::numeric, 2) FROM mv WHERE account_id IS NULL)
            UNION ALL SELECT 'and on the accrual axis',
                (SELECT round(sum(income + expenses)::numeric, 2) FROM ya WHERE account_id IS NULL),
                (SELECT round(sum(income + expenses)::numeric, 2) FROM ma WHERE account_id IS NULL)
            -- and the two axes still agree across a range wide enough to hold everything
            UNION ALL SELECT 'both axes come to what was actually paid',
                (SELECT round(sum(income + expenses)::numeric, 2) FROM ya WHERE account_id IS NULL), 6500.00

            -- an expense counts in every bucket it runs through, which is two years rather
            -- than the twelve months it is
            UNION ALL SELECT 'the subscription counts in both of its years',
                (SELECT count(*) FROM ya WHERE account_id IS NULL AND expense_count >= 1), 2
            UNION ALL SELECT '2024 holds three of them',
                (SELECT expense_count FROM ya WHERE period = y24 AND account_id IS NULL), 3

            -- categories ride on the same movements, so they bucket by the same grain
            UNION ALL SELECT 'the category is split across the years too',
                (SELECT round(expenses::numeric, 2) FROM cy WHERE period = y25 AND category_id = abo), -33400.00
            UNION ALL SELECT 'and holds nothing outside the subscription',
                (SELECT count(*) FROM cy WHERE category_id = abo), 2
        )
        SELECT * FROM checks WHERE actual IS DISTINCT FROM expected
    LOOP
        RAISE WARNING 'FAIL % : got % want %', r.label, r.actual, r.expected;
        failures := failures + 1;
    END LOOP;

    IF failures > 0 THEN
        RAISE EXCEPTION '% granularity check(s) failed', failures;
    END IF;
    RAISE NOTICE 'all granularity checks passed';
END $$;

ROLLBACK;
