-- Covers simulate_balances(): which recorded balance anchors a sample, which direction the
-- expenses in between are applied in, and what the per-account rows add up to.
BEGIN;

INSERT INTO users (id, name, full_name, password_hash) VALUES
    ('00000000-0000-7000-8000-000000000001', 'mine',   'Mine',   'x'),
    ('00000000-0000-7000-8000-000000000002', 'theirs', 'Theirs', 'x');

INSERT INTO accounts (id, user_id, name, description, kind, availability, risk, opened, ownership_fraction) VALUES
    -- s1 carries the ordinary cases
    ('00000000-0000-7000-8000-0000000000a1', '00000000-0000-7000-8000-000000000001', 's1', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0),
    -- s2 has one balance and nothing else, to pin that a lone anchor still reports
    ('00000000-0000-7000-8000-0000000000a2', '00000000-0000-7000-8000-000000000001', 's2', '', 'debit', 'days',        'none', '2025-01-01Z', 1.0),
    -- s3 has no balance and nothing on it either, so it is empty and stays empty
    ('00000000-0000-7000-8000-0000000000a3', '00000000-0000-7000-8000-000000000001', 's3', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0),
    -- s4 belongs to someone else and must not leak into this user's simulation
    ('00000000-0000-7000-8000-0000000000a4', '00000000-0000-7000-8000-000000000002', 's4', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0),
    -- half-owned, to pin that the account's own balance is reported rather than the share of
    -- it that counts towards the owner's wealth
    ('00000000-0000-7000-8000-0000000000a5', '00000000-0000-7000-8000-000000000001', 's5', '', 'debit', 'immediately', 'none', '2025-01-01Z', 0.5),
    -- s6 has no balance either but does have expenses, so its simulation integrates from empty
    ('00000000-0000-7000-8000-0000000000a6', '00000000-0000-7000-8000-000000000001', 's6', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0);

INSERT INTO expenses (id, title, store, description, comments, reference_amount, accrual_start, accrual_end) VALUES
    ('00000000-0000-7000-8000-0000000000e1', 'e1', '', '', '', NULL, '2025-06-05T12:00:00Z', '2025-06-05T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000e2', 'e2', '', '', '', NULL, '2025-06-11T12:00:00Z', '2025-06-11T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000e3', 'e3', '', '', '', NULL, '2025-06-25T12:00:00Z', '2025-06-25T12:00:00Z'),
    -- two transactions of one expense on one account, so the count stays per expense
    ('00000000-0000-7000-8000-0000000000e4', 'e4', '', '', '', NULL, '2025-06-12T12:00:00Z', '2025-06-12T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000e5', 'e5', '', '', '', NULL, '2025-06-10T12:00:00Z', '2025-06-10T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000e6', 'e6', '', '', '', NULL, '2025-06-10T12:00:00Z', '2025-06-10T12:00:00Z'),
    -- falls on the exact instant of b1, which is the boundary both interval ends sit on
    ('00000000-0000-7000-8000-0000000000e7', 'e7', '', '', '', NULL, '2025-06-10T00:00:00Z', '2025-06-10T00:00:00Z'),
    -- touches both users' accounts, so the view renders every one of its transactions once
    -- per involved user and a reader who forgets to filter by user_id counts s1's leg twice
    ('00000000-0000-7000-8000-0000000000e8', 'e8', '', '', '', NULL, '2025-06-11T06:00:00Z', '2025-06-11T06:00:00Z'),
    -- the two on s6, which has nothing but them
    ('00000000-0000-7000-8000-0000000000e9', 'e9', '', '', '', NULL, '2025-06-05T12:00:00Z', '2025-06-05T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000ea', 'ea', '', '', '', NULL, '2025-06-20T12:00:00Z', '2025-06-20T12:00:00Z');

-- A rule's stanza, sitting on s1 inside the forward window. It carries a real transaction with
-- a real date, so nothing but the flag distinguishes it from something that happened.
INSERT INTO expenses (id, title, store, description, comments, reference_amount, accrual_start, accrual_end, is_template) VALUES
    ('00000000-0000-7000-8000-0000000000ef', 'template', '', '', '', NULL, '2025-06-11T18:00:00Z', '2025-06-11T18:00:00Z', true);

INSERT INTO expense_transactions (expense_id, account_id, idx, date, amount, fraction, comments, statement) VALUES
    ('00000000-0000-7000-8000-0000000000e1', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-05T12:00:00Z', -1000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e2', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-11T12:00:00Z',  -500, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e3', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-25T12:00:00Z',  -250, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e4', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-12T12:00:00Z',  -100, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e4', '00000000-0000-7000-8000-0000000000a1', 1, '2025-06-12T12:00:00Z',  -100, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e5', '00000000-0000-7000-8000-0000000000a4', 0, '2025-06-10T12:00:00Z', -9999, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e6', '00000000-0000-7000-8000-0000000000a5', 0, '2025-06-10T12:00:00Z', -1000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e7', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-10T00:00:00Z',  -300, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e8', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-11T06:00:00Z',  -400, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e8', '00000000-0000-7000-8000-0000000000a4', 1, '2025-06-11T06:00:00Z',  -600, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000ef', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-11T18:00:00Z', -7777, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e9', '00000000-0000-7000-8000-0000000000a6', 0, '2025-06-05T12:00:00Z',  2000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000ea', '00000000-0000-7000-8000-0000000000a6', 0, '2025-06-20T12:00:00Z',  -800, NULL, '', '');

INSERT INTO balances (id, account_id, date, amount, comments) VALUES
    ('00000000-0000-7000-8000-0000000000b1', '00000000-0000-7000-8000-0000000000a1', '2025-06-10T00:00:00Z', 5000, ''),
    ('00000000-0000-7000-8000-0000000000b2', '00000000-0000-7000-8000-0000000000a1', '2025-06-20T00:00:00Z', 4400, ''),
    ('00000000-0000-7000-8000-0000000000b3', '00000000-0000-7000-8000-0000000000a2', '2025-06-10T00:00:00Z', 7000, ''),
    ('00000000-0000-7000-8000-0000000000b4', '00000000-0000-7000-8000-0000000000a4', '2025-06-10T00:00:00Z', 3000, ''),
    ('00000000-0000-7000-8000-0000000000b5', '00000000-0000-7000-8000-0000000000a5', '2025-06-01T00:00:00Z', 8000, '');

DO $$
DECLARE
    r record;
    failures int := 0;
    mine uuid := '00000000-0000-7000-8000-000000000001';
    s1 uuid := '00000000-0000-7000-8000-0000000000a1';
    s2 uuid := '00000000-0000-7000-8000-0000000000a2';
    s3 uuid := '00000000-0000-7000-8000-0000000000a3';
    s6 uuid := '00000000-0000-7000-8000-0000000000a6';
BEGIN
    FOR r IN
        WITH checks(label, actual, expected) AS (
            -- On the balance's own date the balance is the answer: it is stated before
            -- anything that happens that same instant, e7 included.
            SELECT 'anchor date reports the balance itself',
                (SELECT amount FROM simulate_balances(mine, ARRAY['2025-06-10T00:00:00Z'::timestamptz], s1)), 5000.0

            -- forward: b1 (5000 on the 10th), then e7 at that same instant, e2 and e8's own
            -- leg on the 11th, and e4's two transactions on the 12th. e8's other leg is on
            -- someone else's account and must not move this one, and the template sitting on
            -- s1 on the 11th must not either.
            UNION ALL SELECT 'forward integration adds what happened since',
                (SELECT amount FROM simulate_balances(mine, ARRAY['2025-06-13T00:00:00Z'::timestamptz], s1)), 3600.0
            UNION ALL SELECT 'the count is of expenses, not transactions',
                (SELECT expense_count FROM count_balance_expenses(mine, '2025-06-13T00:00:00Z', s1)), 4

            -- backward: the nearest balance to the 8th is b1 on the 10th, so e1 on the 5th is
            -- outside the interval and only the emptiness between the 8th and the 10th counts
            UNION ALL SELECT 'a sample before every expense still anchors forward',
                (SELECT amount FROM simulate_balances(mine, ARRAY['2025-06-08T00:00:00Z'::timestamptz], s1)), 5000.0
            -- the 4th is nearest to b1 as well, and e1 on the 5th now falls inside the
            -- interval, so it has to be taken back off
            UNION ALL SELECT 'backward integration removes what happened after',
                (SELECT amount FROM simulate_balances(mine, ARRAY['2025-06-04T00:00:00Z'::timestamptz], s1)), 6000.0
            UNION ALL SELECT 'backward integration counts the same expenses',
                (SELECT expense_count FROM count_balance_expenses(mine, '2025-06-04T00:00:00Z', s1)), 1
            -- the count is anchored the same way the amount is, so an account whose interval
            -- holds nothing still reports a row rather than dropping out
            UNION ALL SELECT 'an empty interval counts nothing',
                (SELECT expense_count FROM count_balance_expenses(mine, '2025-07-01T00:00:00Z', s2)), 0

            -- the 19th is nearer to b2 (the 20th) than to b1 (the 10th), so it anchors there
            -- and takes e3 on the 25th off nothing, leaving b2 alone
            UNION ALL SELECT 'the nearer of two balances wins',
                (SELECT CASE WHEN balance_id = '00000000-0000-7000-8000-0000000000b2' THEN 1 ELSE 0 END
                 FROM simulate_balances(mine, ARRAY['2025-06-19T00:00:00Z'::timestamptz], s1)), 1
            UNION ALL SELECT 'and is used as stated',
                (SELECT amount FROM simulate_balances(mine, ARRAY['2025-06-19T00:00:00Z'::timestamptz], s1)), 4400.0
            -- exactly between the two: the earlier one wins, so the answer is stable
            UNION ALL SELECT 'an exact tie goes to the earlier balance',
                (SELECT CASE WHEN balance_id = '00000000-0000-7000-8000-0000000000b1' THEN 1 ELSE 0 END
                 FROM simulate_balances(mine, ARRAY['2025-06-15T00:00:00Z'::timestamptz], s1)), 1

            -- an account with a balance and no expenses reports it unchanged
            UNION ALL SELECT 's2 has nothing to integrate',
                (SELECT amount FROM simulate_balances(mine, ARRAY['2025-07-01T00:00:00Z'::timestamptz], s2)), 7000.0

            -- an account with no recorded balance anchors on the empty one it started from,
            -- so it reports rather than dropping out, and with nothing on it that is zero
            UNION ALL SELECT 's3 reports nothing rather than not reporting',
                (SELECT amount FROM simulate_balances(mine, ARRAY['2025-06-17T00:00:00Z'::timestamptz], s3)), 0.0
            UNION ALL SELECT 'and names no balance to have been anchored on',
                (SELECT count(*) FROM simulate_balances(mine, ARRAY['2025-06-17T00:00:00Z'::timestamptz], s3)
                 WHERE balance_id IS NULL), 1
            -- the point of the empty anchor: everything on the account is integrated from zero
            UNION ALL SELECT 's6 integrates its expenses from empty',
                (SELECT amount FROM simulate_balances(mine, ARRAY['2025-06-10T00:00:00Z'::timestamptz], s6)), 2000.0
            UNION ALL SELECT 'and keeps integrating past the second one',
                (SELECT amount FROM simulate_balances(mine, ARRAY['2025-06-25T00:00:00Z'::timestamptz], s6)), 1200.0
            -- before anything happened it is still empty, and nothing is taken back off past
            -- the beginning of time
            UNION ALL SELECT 's6 was empty before its first expense',
                (SELECT amount FROM simulate_balances(mine, ARRAY['2025-06-01T00:00:00Z'::timestamptz], s6)), 0.0
            -- and someone else's account is not this user's business
            UNION ALL SELECT 's4 does not appear',
                (SELECT count(*) FROM simulate_balances(mine, ARRAY['2025-06-17T00:00:00Z'::timestamptz])
                 WHERE account_id = '00000000-0000-7000-8000-0000000000a4'), 0
            UNION ALL SELECT 'every account of mine can be anchored',
                (SELECT count(*) FROM simulate_balances(mine, ARRAY['2025-06-17T00:00:00Z'::timestamptz])), 5

            -- a recorded balance is nearer than the beginning of time however far off it is,
            -- so an account that was not empty when it was first written down stays that way
            UNION ALL SELECT 'a real balance always beats the empty one',
                (SELECT amount FROM simulate_balances(mine, ARRAY['2020-01-01T00:00:00Z'::timestamptz], s2)), 7000.0
            UNION ALL SELECT 'and is the balance that gets named',
                (SELECT CASE WHEN balance_id = '00000000-0000-7000-8000-0000000000b3' THEN 1 ELSE 0 END
                 FROM simulate_balances(mine, ARRAY['2020-01-01T00:00:00Z'::timestamptz], s2)), 1

            -- what is reported is the account's balance, not the owner's share of it: the
            -- ownership fraction belongs to effective_amount, which this does not use
            UNION ALL SELECT 'a half-owned account reports its whole balance',
                (SELECT amount FROM simulate_balances(mine, ARRAY['2025-06-11T00:00:00Z'::timestamptz], '00000000-0000-7000-8000-0000000000a5')), 7000.0

            -- many samples in one call, which is the whole point of the array argument
            UNION ALL SELECT 'each sample is answered once per anchorable account',
                (SELECT count(*) FROM simulate_balances(mine,
                    ARRAY['2025-06-04T00:00:00Z', '2025-06-17T00:00:00Z', '2025-07-01T00:00:00Z']::timestamptz[])), 15
            -- the ordinality is the caller's array index, which is what lets the series be
            -- reassembled without matching on the dates again
            UNION ALL SELECT 'samples keep the order they were given in',
                (SELECT sample_idx FROM simulate_balances(mine,
                    ARRAY['2025-06-04T00:00:00Z', '2025-06-13T00:00:00Z']::timestamptz[], s1)
                 WHERE sample_at = '2025-06-04T00:00:00Z'), 1
            UNION ALL SELECT 'and the second sample is the second one',
                (SELECT sample_idx FROM simulate_balances(mine,
                    ARRAY['2025-06-04T00:00:00Z', '2025-06-13T00:00:00Z']::timestamptz[], s1)
                 WHERE sample_at = '2025-06-13T00:00:00Z'), 2
            -- a repeated date is not deduplicated; the caller asked for two points
            UNION ALL SELECT 'a repeated sample is answered twice',
                (SELECT count(*) FROM simulate_balances(mine,
                    ARRAY['2025-06-13T00:00:00Z', '2025-06-13T00:00:00Z']::timestamptz[], s1)), 2
        )
        SELECT * FROM checks WHERE actual IS DISTINCT FROM expected
    LOOP
        RAISE WARNING 'FAIL % : got % want %', r.label, r.actual, r.expected;
        failures := failures + 1;
    END LOOP;

    IF failures > 0 THEN
        RAISE EXCEPTION '% simulation check(s) failed', failures;
    END IF;
    RAISE NOTICE 'all simulation checks passed';
END $$;

ROLLBACK;
