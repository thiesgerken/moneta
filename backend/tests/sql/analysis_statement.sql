-- Covers analysis_statement(): what goes into the ledger, the order it comes out in, and how
-- a recorded balance re-anchors the running total instead of being absorbed by it.
BEGIN;

INSERT INTO users (id, name, full_name, password_hash) VALUES
    ('00000000-0000-7000-8000-000000000001', 'mine',   'Mine',   'x'),
    ('00000000-0000-7000-8000-000000000002', 'theirs', 'Theirs', 'x');

INSERT INTO accounts (id, user_id, name, description, kind, availability, risk, opened, ownership_fraction) VALUES
    ('00000000-0000-7000-8000-0000000000a1', '00000000-0000-7000-8000-000000000001', 'a1', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0),
    -- a second account of the same user, to pin that the ledger is per account
    ('00000000-0000-7000-8000-0000000000a2', '00000000-0000-7000-8000-000000000001', 'a2', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0),
    ('00000000-0000-7000-8000-0000000000a3', '00000000-0000-7000-8000-000000000002', 'a3', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0);

INSERT INTO expenses (id, title, store, description, comments, reference_amount, accrual_start, accrual_end, is_template) VALUES
    -- before the range: reflected in the opening balance, not listed
    ('00000000-0000-7000-8000-0000000000e1', 'before', 'S1', '', '', NULL, '2025-05-20Z', '2025-05-20Z', false),
    ('00000000-0000-7000-8000-0000000000e2', 'rewe',   'Rewe', '', '', NULL, '2025-06-03Z', '2025-06-03Z', false),
    ('00000000-0000-7000-8000-0000000000e3', 'gehalt', 'Arbeit', '', '', NULL, '2025-06-12Z', '2025-06-12Z', false),
    -- two legs on the same account at the same instant: one movement, not two rows
    ('00000000-0000-7000-8000-0000000000e4', 'split',  'Laden', '', '', NULL, '2025-06-18Z', '2025-06-18Z', false),
    -- two legs on the same account at different instants: two rows, like a bank shows them
    ('00000000-0000-7000-8000-0000000000e5', 'raten',  'Werkstatt', '', '', NULL, '2025-06-20Z', '2025-06-20Z', false),
    -- on the other account of the same user, so it must not appear in a1's ledger
    ('00000000-0000-7000-8000-0000000000e6', 'other account', 'X', '', '', NULL, '2025-06-14Z', '2025-06-14Z', false),
    -- someone else's account entirely
    ('00000000-0000-7000-8000-0000000000e7', 'theirs', 'Y', '', '', NULL, '2025-06-14Z', '2025-06-14Z', false),
    -- a stanza a rule copies; it never happened
    ('00000000-0000-7000-8000-0000000000e8', 'template', 'Z', '', '', NULL, '2025-06-15Z', '2025-06-15Z', true),
    -- after the range
    ('00000000-0000-7000-8000-0000000000e9', 'after', 'W', '', '', NULL, '2025-07-10Z', '2025-07-10Z', false),
    -- lands on the exact instant of the recorded balance b3, which is the tie the ordering
    -- rule decides: a balance is stated before the expenses of its own instant
    ('00000000-0000-7000-8000-0000000000ea', 'same instant', 'V', '', '', NULL, '2025-06-19Z', '2025-06-19Z', false);

INSERT INTO expense_transactions (expense_id, account_id, idx, date, amount, fraction, comments, statement) VALUES
    ('00000000-0000-7000-8000-0000000000e1', '00000000-0000-7000-8000-0000000000a1', 0, '2025-05-20Z', -2500, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e2', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-03Z', -4000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e3', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-12Z', 250000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e4', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-18Z', -1200, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e4', '00000000-0000-7000-8000-0000000000a1', 1, '2025-06-18Z',  -800, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e5', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-20Z', -5000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e5', '00000000-0000-7000-8000-0000000000a1', 1, '2025-06-22Z', -3000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e6', '00000000-0000-7000-8000-0000000000a2', 0, '2025-06-14Z', -9999, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e7', '00000000-0000-7000-8000-0000000000a3', 0, '2025-06-14Z', -8888, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e8', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-15Z', -7777, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e9', '00000000-0000-7000-8000-0000000000a1', 0, '2025-07-10Z', -6666, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000ea', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-19Z', -2000, NULL, '', '');

INSERT INTO balances (id, account_id, date, amount, comments) VALUES
    -- anchors the opening balance for the range starting 2025-06-01
    ('00000000-0000-7000-8000-0000000000b1', '00000000-0000-7000-8000-0000000000a1', '2025-05-25Z', 100000, ''),
    -- mid-range and in disagreement: carried in is 100000 - 4000 + 250000 = 346000, recorded
    -- says 345000, so 1000 went missing somewhere
    ('00000000-0000-7000-8000-0000000000b2', '00000000-0000-7000-8000-0000000000a1', '2025-06-15Z', 345000, ''),
    -- agrees exactly with what is carried into it, so drift is zero rather than absent
    ('00000000-0000-7000-8000-0000000000b3', '00000000-0000-7000-8000-0000000000a1', '2025-06-19Z', 343000, '');

DO $$
DECLARE
    r record;
    failures int := 0;
    mine uuid := '00000000-0000-7000-8000-000000000001';
    a1 uuid := '00000000-0000-7000-8000-0000000000a1';
    f timestamptz := '2025-06-01Z';
    t timestamptz := '2025-06-30Z';
BEGIN
    FOR r IN
        WITH s AS (SELECT * FROM analysis_statement(mine, a1, f, t)),
        checks(label, actual, expected) AS (
            -- opening + 6 movements (e4's two legs merge, e5's two dates do not) + 2 balances
            SELECT 'row count', (SELECT count(*) FROM s), 9
            UNION ALL SELECT 'one opening row', (SELECT count(*) FROM s WHERE kind = 'opening'), 1
            UNION ALL SELECT 'two recorded balances', (SELECT count(*) FROM s WHERE kind = 'balance'), 2
            UNION ALL SELECT 'six movements', (SELECT count(*) FROM s WHERE kind = 'expense'), 6

            -- the opening is the simulation at `from`: b1 (100000 on the 25th) with e1 already
            -- inside it, so nothing is left to integrate between the 25th and the 1st
            UNION ALL SELECT 'opening balance', (SELECT balance FROM s WHERE kind = 'opening'), 100000.0
            UNION ALL SELECT 'opening carries no amount',
                (SELECT count(*) FROM s WHERE kind = 'opening' AND amount IS NOT NULL), 0

            -- what happened before the range is inside the opening, not a row of its own
            UNION ALL SELECT 'the earlier expense is not listed',
                (SELECT count(*) FROM s WHERE title = 'before'), 0
            UNION ALL SELECT 'the later expense is not listed',
                (SELECT count(*) FROM s WHERE title = 'after'), 0
            UNION ALL SELECT 'the other account is not listed',
                (SELECT count(*) FROM s WHERE title = 'other account'), 0
            UNION ALL SELECT 'someone else''s account is not listed',
                (SELECT count(*) FROM s WHERE title = 'theirs'), 0
            UNION ALL SELECT 'a template is not listed',
                (SELECT count(*) FROM s WHERE title = 'template'), 0

            -- legs at one instant on one account are one movement
            UNION ALL SELECT 'the split expense is one row',
                (SELECT count(*) FROM s WHERE title = 'split'), 1
            UNION ALL SELECT 'and carries both legs',
                (SELECT amount FROM s WHERE title = 'split'), -2000.0
            -- legs on different days stay separate
            UNION ALL SELECT 'the instalments are two rows',
                (SELECT count(*) FROM s WHERE title = 'raten'), 2

            -- running total: 100000 - 4000 = 96000 after the Rewe purchase
            UNION ALL SELECT 'running total after the first expense',
                (SELECT balance FROM s WHERE title = 'rewe'), 96000.0
            UNION ALL SELECT 'running total after the salary',
                (SELECT balance FROM s WHERE title = 'gehalt'), 346000.0

            -- b2 disagrees with the 346000 carried into it and wins
            UNION ALL SELECT 'a recorded balance re-anchors the total',
                (SELECT balance FROM s WHERE kind = 'balance' AND entry_date = '2025-06-15Z'), 345000.0
            UNION ALL SELECT 'and reports the gap it closed',
                (SELECT drift FROM s WHERE kind = 'balance' AND entry_date = '2025-06-15Z'), -1000.0
            -- everything after it accumulates from the recorded figure, not the carried one
            UNION ALL SELECT 'the next expense continues from the recorded balance',
                (SELECT balance FROM s WHERE title = 'split'), 343000.0

            -- b3 agrees exactly with the 343000 carried into it, and the expense sharing its
            -- instant is stated *after* it, so a flipped tie would show a 2000 drift here
            UNION ALL SELECT 'an agreeing balance drifts by zero',
                (SELECT drift FROM s WHERE kind = 'balance' AND entry_date = '2025-06-19Z'), 0.0
            UNION ALL SELECT 'an expense at the same instant lands after the balance',
                (SELECT balance FROM s WHERE title = 'same instant'), 341000.0
            UNION ALL SELECT 'expense rows carry no drift',
                (SELECT count(*) FROM s WHERE kind = 'expense' AND drift IS NOT NULL), 0

            -- newest first is what you want to read. `emitted` numbers the rows in the order
            -- the function actually hands them over, which is the thing being asserted --
            -- ordering by `seq` here would re-sort them and pass either way.
            UNION ALL SELECT 'the newest row is emitted first',
                (SELECT emitted FROM (
                    SELECT row_number() OVER () AS emitted, seq FROM analysis_statement(mine, a1, f, t)
                 ) o WHERE o.seq = (SELECT max(seq) FROM s)), 1
            UNION ALL SELECT 'the opening row is emitted last',
                (SELECT emitted FROM (
                    SELECT row_number() OVER () AS emitted, seq FROM analysis_statement(mine, a1, f, t)
                 ) o WHERE o.seq = 1), 9
            UNION ALL SELECT 'the opening row is sequenced first',
                (SELECT seq FROM s WHERE kind = 'opening'), 1

            -- the store comes along for the ride, since the view shows it
            UNION ALL SELECT 'the store is carried',
                (SELECT CASE WHEN store = 'Rewe' THEN 1 ELSE 0 END FROM s WHERE title = 'rewe'), 1
        )
        SELECT * FROM checks WHERE actual IS DISTINCT FROM expected
    LOOP
        RAISE WARNING 'FAIL % : got % want %', r.label, r.actual, r.expected;
        failures := failures + 1;
    END LOOP;

    IF failures > 0 THEN
        RAISE EXCEPTION '% statement check(s) failed', failures;
    END IF;
    RAISE NOTICE 'all statement checks passed';
END $$;

ROLLBACK;
