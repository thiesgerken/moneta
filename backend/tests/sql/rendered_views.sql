-- Reproduces the fixtures of the 12 tests in rendering.rs against the views.
BEGIN;

INSERT INTO users (id, name, full_name, password_hash) VALUES
    ('00000000-0000-7000-8000-000000000001', 'mine',   'Mine',   'x'),
    ('00000000-0000-7000-8000-000000000002', 'theirs', 'Theirs', 'x');

INSERT INTO accounts (id, user_id, name, description, kind, availability, risk, opened, ownership_fraction) VALUES
    ('00000000-0000-7000-8000-0000000000a1', '00000000-0000-7000-8000-000000000001', 'a1', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0),
    ('00000000-0000-7000-8000-0000000000a2', '00000000-0000-7000-8000-000000000001', 'a2', '', 'debit', 'immediately', 'none', '2025-01-01Z', 0.5),
    ('00000000-0000-7000-8000-0000000000a3', '00000000-0000-7000-8000-000000000001', 'a3', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0),
    ('00000000-0000-7000-8000-0000000000a4', '00000000-0000-7000-8000-000000000002', 'a4', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0),
    ('00000000-0000-7000-8000-0000000000a5', '00000000-0000-7000-8000-000000000001', 'a5', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0),
    ('00000000-0000-7000-8000-0000000000a6', '00000000-0000-7000-8000-000000000002', 'a6', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0),
    ('00000000-0000-7000-8000-0000000000a7', '00000000-0000-7000-8000-000000000002', 'a7', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0),
    -- a8/a9 mirror with different ownership on each side, so a rendering that reached for the
    -- wrong account's ownership_fraction would still look plausible on a1..a7
    ('00000000-0000-7000-8000-0000000000a8', '00000000-0000-7000-8000-000000000001', 'a8', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0),
    ('00000000-0000-7000-8000-0000000000a9', '00000000-0000-7000-8000-000000000002', 'a9', '', 'debit', 'immediately', 'none', '2025-01-01Z', 0.5),
    -- two accounts of the same user, to pin that an expense across both renders once
    ('00000000-0000-7000-8000-0000000000aa', '00000000-0000-7000-8000-000000000001', 'aa', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0),
    ('00000000-0000-7000-8000-0000000000ab', '00000000-0000-7000-8000-000000000001', 'ab', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0);

-- mirrors have to be linked after the fact; the reference is mutual
UPDATE accounts SET sync_account_id = '00000000-0000-7000-8000-0000000000a4', sync_invert = true
    WHERE id = '00000000-0000-7000-8000-0000000000a3';
UPDATE accounts SET sync_account_id = '00000000-0000-7000-8000-0000000000a3', sync_invert = true
    WHERE id = '00000000-0000-7000-8000-0000000000a4';
UPDATE accounts SET sync_account_id = '00000000-0000-7000-8000-0000000000a6', sync_invert = false
    WHERE id = '00000000-0000-7000-8000-0000000000a5';
UPDATE accounts SET sync_account_id = '00000000-0000-7000-8000-0000000000a5', sync_invert = false
    WHERE id = '00000000-0000-7000-8000-0000000000a6';
UPDATE accounts SET sync_account_id = '00000000-0000-7000-8000-0000000000a9', sync_invert = true
    WHERE id = '00000000-0000-7000-8000-0000000000a8';
UPDATE accounts SET sync_account_id = '00000000-0000-7000-8000-0000000000a8', sync_invert = true
    WHERE id = '00000000-0000-7000-8000-0000000000a9';

INSERT INTO categories (id, name, description) VALUES
    ('00000000-0000-7000-8000-0000000000c1', 'Lebensmittel', ''),
    ('00000000-0000-7000-8000-0000000000c2', 'Haushalt', '');

INSERT INTO expenses (id, title, store, description, comments, reference_amount, booking_start, booking_end) VALUES
    ('00000000-0000-7000-8000-0000000000e1', 'e1', '', '', '', NULL,  '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000e2', 'e2', '', '', '', NULL,  '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000e3', 'e3', '', '', '', -2000, '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000e4', 'e4', '', '', '', NULL,  '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000e5', 'e5', '', '', '', NULL,  '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000e6', 'e6', '', '', '', NULL,  '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000e7', 'e7', '', '', '', NULL,  '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z'),
    -- a fractional transaction with nothing to resolve against; rendering it is impossible
    ('00000000-0000-7000-8000-0000000000e8', 'e8', '', '', '', NULL,  '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000e9', 'e9', '', '', '', NULL,  '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000ea', 'ea', '', '', '', -2000, '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000eb', 'eb', '', '', '', -2000, '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000ec', 'ec', '', '', '', NULL,  '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000ed', 'ed', '', '', '', NULL,  '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000ee', 'ee', '', '', '', NULL,  '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z');

INSERT INTO expense_categorizations (expense_id, category_id, weight) VALUES
    ('00000000-0000-7000-8000-0000000000ec', '00000000-0000-7000-8000-0000000000c1', 0.75),
    ('00000000-0000-7000-8000-0000000000ec', '00000000-0000-7000-8000-0000000000c2', 0.25);

INSERT INTO expense_transactions (expense_id, account_id, idx, date, amount, fraction, comments, statement) VALUES
    ('00000000-0000-7000-8000-0000000000e1', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-01T12:00:00Z', -1250, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e2', '00000000-0000-7000-8000-0000000000a2', 0, '2025-06-01T12:00:00Z', -1000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e3', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-01T12:00:00Z', NULL, 0.25, '', ''),
    ('00000000-0000-7000-8000-0000000000e4', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-01T12:00:00Z', -1000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e4', '00000000-0000-7000-8000-0000000000a1', 1, '2025-06-01T12:00:00Z',  -500, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e5', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-01T12:00:00Z', -1000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e5', '00000000-0000-7000-8000-0000000000a7', 1, '2025-06-01T12:00:00Z',  -400, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e6', '00000000-0000-7000-8000-0000000000a3', 0, '2025-06-01T12:00:00Z', -1000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e7', '00000000-0000-7000-8000-0000000000a5', 0, '2025-06-01T12:00:00Z', -1000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e8', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-01T12:00:00Z', NULL, 0.25, '', ''),

    -- inserted with the higher idx first, so a view that returned them in physical order
    -- rather than by idx would come back the wrong way round
    ('00000000-0000-7000-8000-0000000000e9', '00000000-0000-7000-8000-0000000000a1', 1, '2025-06-01T12:00:00Z',  -111, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e9', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-01T12:00:00Z',  -999, NULL, '', ''),

    -- a fixed and a fractional transaction on one expense, the realistic split
    ('00000000-0000-7000-8000-0000000000ea', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-01T12:00:00Z', -1000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000ea', '00000000-0000-7000-8000-0000000000a1', 1, '2025-06-01T12:00:00Z',  NULL, 0.5,  '', ''),

    -- fractional on the inverted mirror: the sign flip has to reach `fraction` too
    ('00000000-0000-7000-8000-0000000000eb', '00000000-0000-7000-8000-0000000000a8', 0, '2025-06-01T12:00:00Z',  NULL, 0.25, '', ''),

    ('00000000-0000-7000-8000-0000000000ec', '00000000-0000-7000-8000-0000000000aa', 0, '2025-06-01T12:00:00Z',  -100, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000ed', '00000000-0000-7000-8000-0000000000aa', 0, '2025-06-01T12:00:00Z',  -100, NULL, '', ''),

    -- one expense across two accounts of the same user still renders once
    ('00000000-0000-7000-8000-0000000000ee', '00000000-0000-7000-8000-0000000000aa', 0, '2025-06-01T12:00:00Z',  -100, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000ee', '00000000-0000-7000-8000-0000000000ab', 1, '2025-06-01T12:00:00Z',  -200, NULL, '', '');

INSERT INTO balances (id, account_id, date, amount, comments) VALUES
    ('00000000-0000-7000-8000-0000000000b1', '00000000-0000-7000-8000-0000000000a1', '2025-06-01T12:00:00Z', 5000, ''),
    ('00000000-0000-7000-8000-0000000000b2', '00000000-0000-7000-8000-0000000000a3', '2025-06-01T12:00:00Z', 5000, ''),
    -- a5/a6 mirror without inversion, so the balance keeps its sign on the other side
    ('00000000-0000-7000-8000-0000000000b3', '00000000-0000-7000-8000-0000000000a5', '2025-06-01T12:00:00Z', 5000, '');

DO $$
DECLARE
    r record;
    failures int := 0;
BEGIN
    FOR r IN
        WITH checks(label, actual, expected) AS (
            -- renders_a_wholly_owned_expense_at_face_value
            SELECT 'e1 effective', (SELECT effective_amount FROM expenses_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000e1'), -1250.0
            UNION ALL SELECT 'e1 rows', (SELECT count(*) FROM expenses_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000e1'), 1
            UNION ALL SELECT 'e1 calculated', (SELECT calculated_amount FROM expense_transactions_rendered WHERE expense_id = '00000000-0000-7000-8000-0000000000e1'), -1250.0

            -- applies_the_ownership_fraction_to_the_effective_amount
            UNION ALL SELECT 'e2 calculated', (SELECT calculated_amount FROM expense_transactions_rendered WHERE expense_id = '00000000-0000-7000-8000-0000000000e2'), -1000.0
            UNION ALL SELECT 'e2 effective', (SELECT effective_amount FROM expenses_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000e2'), -500.0

            -- resolves_fractional_transactions_against_the_reference_amount
            UNION ALL SELECT 'e3 calculated', (SELECT calculated_amount FROM expense_transactions_rendered WHERE expense_id = '00000000-0000-7000-8000-0000000000e3'), -500.0

            -- sums_effective_amounts_across_transactions
            UNION ALL SELECT 'e4 effective', (SELECT effective_amount FROM expenses_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000e4'), -1500.0

            -- renders_one_document_per_involved_user
            UNION ALL SELECT 'e5 rows', (SELECT count(*) FROM expenses_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000e5'), 2
            UNION ALL SELECT 'e5 for mine', (SELECT effective_amount FROM expenses_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000e5' AND user_id = '00000000-0000-7000-8000-000000000001'), -1000.0
            UNION ALL SELECT 'e5 for theirs', (SELECT effective_amount FROM expenses_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000e5' AND user_id = '00000000-0000-7000-8000-000000000002'), -400.0

            -- inverted_synchronization_flips_the_sign_for_the_other_user
            UNION ALL SELECT 'e6 rows', (SELECT count(*) FROM expenses_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000e6'), 2
            UNION ALL SELECT 'e6 for mine', (SELECT effective_amount FROM expenses_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000e6' AND user_id = '00000000-0000-7000-8000-000000000001'), -1000.0
            UNION ALL SELECT 'e6 for theirs', (SELECT effective_amount FROM expenses_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000e6' AND user_id = '00000000-0000-7000-8000-000000000002'), 1000.0
            UNION ALL SELECT 'e6 their amount', (SELECT amount FROM expense_transactions_rendered WHERE expense_id = '00000000-0000-7000-8000-0000000000e6' AND user_id = '00000000-0000-7000-8000-000000000002'), 1000
            UNION ALL SELECT 'e6 their account is a4', (SELECT CASE WHEN account_id = '00000000-0000-7000-8000-0000000000a4' THEN 1 ELSE 0 END FROM expense_transactions_rendered WHERE expense_id = '00000000-0000-7000-8000-0000000000e6' AND user_id = '00000000-0000-7000-8000-000000000002'), 1

            -- plain_synchronization_keeps_the_sign_for_the_other_user
            UNION ALL SELECT 'e7 for theirs', (SELECT effective_amount FROM expenses_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000e7' AND user_id = '00000000-0000-7000-8000-000000000002'), -1000.0

            -- rejects_a_fractional_transaction_without_a_reference_amount: nothing to resolve
            -- against has to stay null all the way up, so the Rust side errors on the read
            -- rather than reading a transaction that silently contributes nothing
            UNION ALL SELECT 'e8 calculated is null', (SELECT CASE WHEN calculated_amount IS NULL THEN 1 ELSE 0 END FROM expense_transactions_rendered WHERE expense_id = '00000000-0000-7000-8000-0000000000e8'), 1
            UNION ALL SELECT 'e8 effective is null', (SELECT CASE WHEN effective_amount IS NULL THEN 1 ELSE 0 END FROM expenses_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000e8'), 1

            -- a transaction on an account the reader neither owns nor mirrors still shows up
            -- in their copy of the expense, but must not move their wealth
            UNION ALL SELECT 'e5 foreign transaction is weightless', (SELECT effective_amount FROM expense_transactions_rendered WHERE expense_id = '00000000-0000-7000-8000-0000000000e5' AND user_id = '00000000-0000-7000-8000-000000000001' AND idx = 1), 0.0
            UNION ALL SELECT 'e5 still lists both transactions', (SELECT count(*) FROM expense_transactions_rendered WHERE expense_id = '00000000-0000-7000-8000-0000000000e5' AND user_id = '00000000-0000-7000-8000-000000000001'), 2

            -- a mixed fixed/fractional expense: the fraction resolves against the expense's
            -- reference amount, not against the sibling transaction
            UNION ALL SELECT 'ea fixed leg', (SELECT calculated_amount FROM expense_transactions_rendered WHERE expense_id = '00000000-0000-7000-8000-0000000000ea' AND idx = 0), -1000.0
            UNION ALL SELECT 'ea fractional leg', (SELECT calculated_amount FROM expense_transactions_rendered WHERE expense_id = '00000000-0000-7000-8000-0000000000ea' AND idx = 1), -1000.0
            UNION ALL SELECT 'ea total', (SELECT effective_amount FROM expenses_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000ea'), -2000.0

            -- the mirror side has to use its own ownership fraction, not the originating
            -- account's: a9 is owned 0.5 while a8 is owned wholly
            UNION ALL SELECT 'eb for mine', (SELECT effective_amount FROM expenses_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000eb' AND user_id = '00000000-0000-7000-8000-000000000001'), -500.0
            UNION ALL SELECT 'eb for theirs uses their ownership', (SELECT effective_amount FROM expenses_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000eb' AND user_id = '00000000-0000-7000-8000-000000000002'), 250.0
            UNION ALL SELECT 'eb their calculated is flipped', (SELECT calculated_amount FROM expense_transactions_rendered WHERE expense_id = '00000000-0000-7000-8000-0000000000eb' AND user_id = '00000000-0000-7000-8000-000000000002'), 500.0
            -- the reported fraction flips with it, so their copy reads consistently
            UNION ALL SELECT 'eb their fraction is flipped', (SELECT fraction FROM expense_transactions_rendered WHERE expense_id = '00000000-0000-7000-8000-0000000000eb' AND user_id = '00000000-0000-7000-8000-000000000002'), -0.25
            UNION ALL SELECT 'eb my fraction is untouched', (SELECT fraction FROM expense_transactions_rendered WHERE expense_id = '00000000-0000-7000-8000-0000000000eb' AND user_id = '00000000-0000-7000-8000-000000000001'), 0.25

            -- two accounts of one user is still one rendering, not one per account
            UNION ALL SELECT 'ee renders once', (SELECT count(*) FROM expenses_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000ee'), 1
            UNION ALL SELECT 'ee total', (SELECT effective_amount FROM expenses_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000ee'), -300.0

            -- plain (uninverted) synchronization leaves a balance's sign alone
            UNION ALL SELECT 'b3 rows', (SELECT count(*) FROM balances_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000b3'), 2
            UNION ALL SELECT 'b3 for mine', (SELECT amount FROM balances_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000b3' AND user_id = '00000000-0000-7000-8000-000000000001'), 5000
            UNION ALL SELECT 'b3 for theirs', (SELECT amount FROM balances_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000b3' AND user_id = '00000000-0000-7000-8000-000000000002'), 5000
            -- and reports it against the account in their own books
            UNION ALL SELECT 'b3 theirs is their account', (SELECT CASE WHEN account_id = '00000000-0000-7000-8000-0000000000a6' THEN 1 ELSE 0 END FROM balances_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000b3' AND user_id = '00000000-0000-7000-8000-000000000002'), 1

            -- renders_a_balance_for_its_owner
            UNION ALL SELECT 'b1 rows', (SELECT count(*) FROM balances_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000b1'), 1
            UNION ALL SELECT 'b1 amount', (SELECT amount FROM balances_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000b1'), 5000

            -- inverted_synchronization_flips_the_balance_for_the_other_user
            UNION ALL SELECT 'b2 rows', (SELECT count(*) FROM balances_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000b2'), 2
            UNION ALL SELECT 'b2 for mine', (SELECT amount FROM balances_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000b2' AND user_id = '00000000-0000-7000-8000-000000000001'), 5000
            UNION ALL SELECT 'b2 for theirs', (SELECT amount FROM balances_rendered WHERE upstream_id = '00000000-0000-7000-8000-0000000000b2' AND user_id = '00000000-0000-7000-8000-000000000002'), -5000
        )
        SELECT * FROM checks WHERE actual IS DISTINCT FROM expected
    LOOP
        RAISE WARNING 'FAIL % : got % want %', r.label, r.actual, r.expected;
        failures := failures + 1;
    END LOOP;

    IF failures > 0 THEN
        RAISE EXCEPTION '% view check(s) failed', failures;
    END IF;
    RAISE NOTICE 'all view checks passed';
END $$;

-- The aggregated transaction list is deserialized straight into the Rust structs, so its keys
-- have to stay camelCase and its order has to follow the stored idx rather than whatever the
-- planner returns.
DO $$
DECLARE
    tx jsonb;
BEGIN
    SELECT transactions INTO tx FROM expenses_rendered
    WHERE upstream_id = '00000000-0000-7000-8000-0000000000e4';

    IF (SELECT array_agg(k ORDER BY k) FROM jsonb_object_keys(tx -> 0) k)
        <> ARRAY['accountId', 'amount', 'calculatedAmount', 'comments', 'date',
                 'effectiveAmount', 'fraction', 'statement']::text[]
    THEN
        RAISE EXCEPTION 'unexpected transaction keys: %',
            (SELECT array_agg(k ORDER BY k) FROM jsonb_object_keys(tx -> 0) k);
    END IF;

    IF (tx -> 0 ->> 'amount')::int <> -1000 OR (tx -> 1 ->> 'amount')::int <> -500 THEN
        RAISE EXCEPTION 'transactions came back out of idx order: %', tx;
    END IF;

    IF (tx -> 0 ->> 'date') !~ '^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}' THEN
        RAISE EXCEPTION 'date is not rfc3339: %', tx -> 0 ->> 'date';
    END IF;

    RAISE NOTICE 'transaction json shape ok';
END $$;

-- The aggregated lists are deserialized straight into Vec fields, so an expense with no
-- categorizations has to come back as an empty array rather than a json null.
DO $$
DECLARE
    cats jsonb;
    tx jsonb;
BEGIN
    SELECT categories INTO cats FROM expenses_rendered
    WHERE upstream_id = '00000000-0000-7000-8000-0000000000ec';

    IF jsonb_array_length(cats) <> 2 THEN
        RAISE EXCEPTION 'expected two categorizations, got %', cats;
    END IF;
    IF (SELECT array_agg(k ORDER BY k) FROM jsonb_object_keys(cats -> 0) k)
        <> ARRAY['categoryId', 'weight']::text[]
    THEN
        RAISE EXCEPTION 'unexpected categorization keys: %', cats -> 0;
    END IF;
    IF (SELECT (c ->> 'weight')::float FROM jsonb_array_elements(cats) c
        WHERE c ->> 'categoryId' = '00000000-0000-7000-8000-0000000000c1') <> 0.75
    THEN
        RAISE EXCEPTION 'categorization weight did not survive: %', cats;
    END IF;

    SELECT categories INTO cats FROM expenses_rendered
    WHERE upstream_id = '00000000-0000-7000-8000-0000000000ed';
    IF cats <> '[]'::jsonb THEN
        RAISE EXCEPTION 'an uncategorized expense should render [], got %', cats;
    END IF;

    -- ordering by idx, not by physical row order: e9's transactions were inserted backwards
    SELECT transactions INTO tx FROM expenses_rendered
    WHERE upstream_id = '00000000-0000-7000-8000-0000000000e9';
    IF (tx -> 0 ->> 'amount')::int <> -999 OR (tx -> 1 ->> 'amount')::int <> -111 THEN
        RAISE EXCEPTION 'transactions were not ordered by idx: %', tx;
    END IF;

    RAISE NOTICE 'aggregated list shapes ok';
END $$;

-- receipts and positions are passed through untouched, and default to empty arrays
DO $$
DECLARE
    r record;
BEGIN
    UPDATE expenses SET
        receipts = '[{"uri": "file:///beleg.pdf", "ocr": null}]',
        positions = '[{"amount": -100, "quantity": 2.0, "unit": "kg", "title": "Mehl", "comments": ""}]'
    WHERE id = '00000000-0000-7000-8000-0000000000ec';

    SELECT receipts, positions INTO r FROM expenses_rendered
    WHERE upstream_id = '00000000-0000-7000-8000-0000000000ec';

    IF r.receipts -> 0 ->> 'uri' <> 'file:///beleg.pdf' THEN
        RAISE EXCEPTION 'receipts did not survive the view: %', r.receipts;
    END IF;
    IF (r.positions -> 0 ->> 'quantity')::float <> 2.0 THEN
        RAISE EXCEPTION 'positions did not survive the view: %', r.positions;
    END IF;

    SELECT receipts, positions INTO r FROM expenses_rendered
    WHERE upstream_id = '00000000-0000-7000-8000-0000000000ed';
    IF r.receipts <> '[]'::jsonb OR r.positions <> '[]'::jsonb THEN
        RAISE EXCEPTION 'expected empty arrays by default, got % / %', r.receipts, r.positions;
    END IF;

    RAISE NOTICE 'document passthrough ok';
END $$;

ROLLBACK;
