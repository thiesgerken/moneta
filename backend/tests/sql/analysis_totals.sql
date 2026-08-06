-- Covers analysis_month_totals, analysis_category_totals and analysis_category_expenses:
-- which movements count, how they split across months, and how a categorization weight
-- divides one expense between categories without losing or inventing any of it.
BEGIN;

INSERT INTO users (id, name, full_name, password_hash) VALUES
    ('00000000-0000-7000-8000-000000000001', 'mine',   'Mine',   'x'),
    ('00000000-0000-7000-8000-000000000002', 'theirs', 'Theirs', 'x');

INSERT INTO accounts (id, user_id, name, description, kind, availability, risk, opened, ownership_fraction) VALUES
    ('00000000-0000-7000-8000-0000000000a1', '00000000-0000-7000-8000-000000000001', 'a1', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0),
    ('00000000-0000-7000-8000-0000000000a2', '00000000-0000-7000-8000-000000000001', 'a2', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0),
    -- half-owned: the totals report what moved on the account, not the reader's share of it,
    -- because they have to reconcile with the balances
    ('00000000-0000-7000-8000-0000000000a3', '00000000-0000-7000-8000-000000000001', 'a3', '', 'debit', 'immediately', 'none', '2025-01-01Z', 0.5),
    ('00000000-0000-7000-8000-0000000000a9', '00000000-0000-7000-8000-000000000002', 'a9', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0);

INSERT INTO categories (id, name, description) VALUES
    ('00000000-0000-7000-8000-0000000000c1', 'Essen', ''),
    ('00000000-0000-7000-8000-0000000000c2', 'Haushalt', '');

INSERT INTO expenses (id, title, store, description, comments, reference_amount, booking_start, booking_end, is_template) VALUES
    ('00000000-0000-7000-8000-0000000000e1', 'juni klein',  '', '', '', NULL, '2025-06-03Z', '2025-06-03Z', false),
    ('00000000-0000-7000-8000-0000000000e2', 'juni gross',  '', '', '', NULL, '2025-06-20Z', '2025-06-20Z', false),
    ('00000000-0000-7000-8000-0000000000e3', 'juni gehalt', '', '', '', NULL, '2025-06-12Z', '2025-06-12Z', false),
    ('00000000-0000-7000-8000-0000000000e4', 'juli',        '', '', '', NULL, '2025-07-05Z', '2025-07-05Z', false),
    -- split 0.75/0.25 across two categories
    ('00000000-0000-7000-8000-0000000000e5', 'geteilt',     '', '', '', NULL, '2025-06-08Z', '2025-06-08Z', false),
    -- no categorization at all, so it lands under a null category rather than vanishing
    ('00000000-0000-7000-8000-0000000000e6', 'unsortiert',  '', '', '', NULL, '2025-06-09Z', '2025-06-09Z', false),
    -- two transactions in one month on one account: one expense, not two
    ('00000000-0000-7000-8000-0000000000e7', 'zweiteilig',  '', '', '', NULL, '2025-06-10Z', '2025-06-10Z', false),
    -- entirely on someone else's account
    ('00000000-0000-7000-8000-0000000000e8', 'fremd',       '', '', '', NULL, '2025-06-11Z', '2025-06-11Z', false),
    ('00000000-0000-7000-8000-0000000000e9', 'template',    '', '', '', NULL, '2025-06-13Z', '2025-06-13Z', true),
    -- on the half-owned account
    ('00000000-0000-7000-8000-0000000000ea', 'halb',        '', '', '', NULL, '2025-06-14Z', '2025-06-14Z', false),
    -- touches both users' accounts. The view renders every leg once per involved user, so
    -- this user's copy also lists the leg on a9 -- counting it would be someone else's money.
    ('00000000-0000-7000-8000-0000000000eb', 'geteilt mit', '', '', '', NULL, '2025-06-16Z', '2025-06-16Z', false),
    -- a transfer between two of this user's own accounts: one expense, two account rows. This
    -- is what makes a summed per-account count disagree with a distinct one.
    ('00000000-0000-7000-8000-0000000000ec', 'umbuchung',   '', '', '', NULL, '2025-06-17Z', '2025-06-17Z', false);

INSERT INTO expense_categorizations (expense_id, category_id, weight) VALUES
    ('00000000-0000-7000-8000-0000000000e1', '00000000-0000-7000-8000-0000000000c1', 1.0),
    ('00000000-0000-7000-8000-0000000000e2', '00000000-0000-7000-8000-0000000000c1', 1.0),
    ('00000000-0000-7000-8000-0000000000e5', '00000000-0000-7000-8000-0000000000c1', 0.75),
    ('00000000-0000-7000-8000-0000000000e5', '00000000-0000-7000-8000-0000000000c2', 0.25);

INSERT INTO expense_transactions (expense_id, account_id, idx, date, amount, fraction, comments, statement) VALUES
    ('00000000-0000-7000-8000-0000000000e1', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-03Z',  -1000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e2', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-20Z', -50000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e3', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-12Z', 200000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e4', '00000000-0000-7000-8000-0000000000a1', 0, '2025-07-05Z',  -3000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e5', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-08Z',  -4000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e6', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-09Z',  -2000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e7', '00000000-0000-7000-8000-0000000000a2', 0, '2025-06-10Z',   -600, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e7', '00000000-0000-7000-8000-0000000000a2', 1, '2025-06-11Z',   -400, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e8', '00000000-0000-7000-8000-0000000000a9', 0, '2025-06-11Z',  -9999, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e9', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-13Z',  -7777, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000ea', '00000000-0000-7000-8000-0000000000a3', 0, '2025-06-14Z',  -8000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000eb', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-16Z',   -700, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000eb', '00000000-0000-7000-8000-0000000000a9', 1, '2025-06-16Z',  -1300, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000ec', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-17Z',  -5000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000ec', '00000000-0000-7000-8000-0000000000a2', 1, '2025-06-17Z',   5000, NULL, '', '');

DO $$
DECLARE
    r record;
    failures int := 0;
    mine uuid := '00000000-0000-7000-8000-000000000001';
    a1 uuid := '00000000-0000-7000-8000-0000000000a1';
    a2 uuid := '00000000-0000-7000-8000-0000000000a2';
    a3 uuid := '00000000-0000-7000-8000-0000000000a3';
    c1 uuid := '00000000-0000-7000-8000-0000000000c1';
    c2 uuid := '00000000-0000-7000-8000-0000000000c2';
    jun timestamptz := '2025-06-01Z';
    aug timestamptz := '2025-08-01Z';
BEGIN
    FOR r IN
        WITH m AS (SELECT * FROM analysis_month_totals(mine, jun, aug)),
             c AS (SELECT * FROM analysis_category_totals(mine, jun, aug)),
             x AS (SELECT * FROM analysis_category_expenses(mine, jun, aug, 2)),
        checks(label, actual, expected) AS (
            -- June on a1: -1000, -50000, +200000, -4000 (split), -2000 (uncategorized).
            -- The template and the other accounts are elsewhere.
            SELECT 'june income on a1',
                (SELECT income FROM m WHERE month = jun AND account_id = a1), 200000.0
            UNION ALL SELECT 'june spending on a1',
                (SELECT expenses FROM m WHERE month = jun AND account_id = a1), -62700.0
            UNION ALL SELECT 'june expense count on a1',
                (SELECT expense_count FROM m WHERE month = jun AND account_id = a1), 7
            -- the shared expense contributes only its own leg; the other user's -1300 is
            -- rendered into this user's copy but never moved this user's money
            UNION ALL SELECT 'a shared expense contributes only this side',
                (SELECT count(*) FROM m WHERE account_id IS NOT NULL AND expenses = -59000.0), 0

            -- one expense with two transactions in the month is counted once
            UNION ALL SELECT 'two transactions of one expense count once',
                (SELECT expense_count FROM m WHERE month = jun AND account_id = a2), 2
            UNION ALL SELECT 'but both amounts are summed',
                (SELECT expenses FROM m WHERE month = jun AND account_id = a2), -1000.0

            -- what moved on the account, not the reader's half of it: these totals have to
            -- reconcile with the balances, which are account balances
            UNION ALL SELECT 'a half-owned account reports the whole movement',
                (SELECT expenses FROM m WHERE month = jun AND account_id = a3), -8000.0

            -- July is its own row, and only holds July
            UNION ALL SELECT 'july is separated out',
                (SELECT expenses FROM m WHERE month = '2025-07-01Z' AND account_id = a1), -3000.0
            UNION ALL SELECT 'two months, three account rows in june and one in july',
                (SELECT count(*) FROM m WHERE account_id IS NOT NULL), 4
            -- plus one roll-up row per month
            UNION ALL SELECT 'each month rolls up once',
                (SELECT count(*) FROM m WHERE account_id IS NULL), 2

            -- The roll-up counts an expense once however many of your accounts it touches.
            -- ec is a transfer booking against both a1 and a2, so the per-account rows add up
            -- to one more than the month actually holds.
            UNION ALL SELECT 'the month counts each expense once',
                (SELECT expense_count FROM m WHERE month = jun AND account_id IS NULL), 9
            UNION ALL SELECT 'which is fewer than the per-account counts summed',
                (SELECT sum(expense_count) FROM m WHERE month = jun AND account_id IS NOT NULL), 10
            -- the money still adds up the same either way
            UNION ALL SELECT 'the roll-up sums the same spending',
                (SELECT expenses FROM m WHERE month = jun AND account_id IS NULL),
                (SELECT sum(expenses) FROM m WHERE month = jun AND account_id IS NOT NULL)
            UNION ALL SELECT 'and the same income',
                (SELECT income FROM m WHERE month = jun AND account_id IS NULL),
                (SELECT sum(income) FROM m WHERE month = jun AND account_id IS NOT NULL)

            -- a template never happened, and someone else's account is not this user's money
            UNION ALL SELECT 'no template anywhere',
                (SELECT coalesce(sum(expenses), 0) FROM m WHERE expenses = -7777), 0
            UNION ALL SELECT 'no foreign account row',
                (SELECT count(*) FROM m WHERE account_id = '00000000-0000-7000-8000-0000000000a9'), 0

            -- categories: c1 gets e1 (-1000), e2 (-50000) and 0.75 of e5 (-3000)
            UNION ALL SELECT 'june spending in c1',
                (SELECT expenses FROM c WHERE month = jun AND category_id = c1), -54000.0
            -- c2 gets only the remaining quarter of e5
            UNION ALL SELECT 'june spending in c2',
                (SELECT expenses FROM c WHERE month = jun AND category_id = c2), -1000.0
            -- A transfer between two of your own accounts nets to zero across the expense, so
            -- it lands in neither bucket -- moving money is not spending it. That is why ec
            -- does not show up in the two figures below even though it is uncategorized.
            UNION ALL SELECT 'a transfer is neither income nor spending',
                (SELECT count(*) FROM x WHERE expense_id = '00000000-0000-7000-8000-0000000000ec'), 0

            -- uncategorized expenses land under a null category rather than being dropped:
            -- e6 on a1, e7's two legs on a2, ea on a3 and eb's own leg
            UNION ALL SELECT 'uncategorized spending is kept',
                (SELECT expenses FROM c WHERE month = jun AND category_id IS NULL), -11700.0
            -- income is split out from spending rather than netted against it
            UNION ALL SELECT 'income is not netted against spending',
                (SELECT income FROM c WHERE month = jun AND category_id IS NULL), 200000.0

            -- and the categories add back up to the accounts
            UNION ALL SELECT 'every category share sums to the month',
                (SELECT round((
                    (SELECT sum(income + expenses) FROM c WHERE month = jun)
                  - (SELECT income + expenses FROM m WHERE month = jun AND account_id IS NULL)
                )::numeric)), 0

            -- the treemap's inner level: the two biggest of c1 by absolute amount, so the
            -- -1000 one drops out behind -50000 and -3000
            UNION ALL SELECT 'two biggest in c1', (SELECT count(*) FROM x WHERE category_id = c1), 2
            UNION ALL SELECT 'the biggest of c1',
                (SELECT min(amount) FROM x WHERE category_id = c1), -50000.0
            UNION ALL SELECT 'the runner-up is the weighted share, not the whole expense',
                (SELECT amount FROM x WHERE category_id = c1 AND expense_id = '00000000-0000-7000-8000-0000000000e5'), -3000.0
            UNION ALL SELECT 'the small one is left for the rest slice',
                (SELECT count(*) FROM x WHERE category_id = c1 AND expense_id = '00000000-0000-7000-8000-0000000000e1'), 0
            UNION ALL SELECT 'titles come along',
                (SELECT CASE WHEN title = 'juni gross' THEN 1 ELSE 0 END FROM x WHERE amount = -50000.0), 1
        )
        SELECT * FROM checks WHERE actual IS DISTINCT FROM expected
    LOOP
        RAISE WARNING 'FAIL % : got % want %', r.label, r.actual, r.expected;
        failures := failures + 1;
    END LOOP;

    IF failures > 0 THEN
        RAISE EXCEPTION '% totals check(s) failed', failures;
    END IF;
    RAISE NOTICE 'all totals checks passed';
END $$;

ROLLBACK;
