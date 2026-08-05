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
    ('00000000-0000-7000-8000-0000000000a7', '00000000-0000-7000-8000-000000000002', 'a7', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0);

-- mirrors have to be linked after the fact; the reference is mutual
UPDATE accounts SET sync_account_id = '00000000-0000-7000-8000-0000000000a4', sync_invert = true
    WHERE id = '00000000-0000-7000-8000-0000000000a3';
UPDATE accounts SET sync_account_id = '00000000-0000-7000-8000-0000000000a3', sync_invert = true
    WHERE id = '00000000-0000-7000-8000-0000000000a4';
UPDATE accounts SET sync_account_id = '00000000-0000-7000-8000-0000000000a6', sync_invert = false
    WHERE id = '00000000-0000-7000-8000-0000000000a5';
UPDATE accounts SET sync_account_id = '00000000-0000-7000-8000-0000000000a5', sync_invert = false
    WHERE id = '00000000-0000-7000-8000-0000000000a6';

INSERT INTO expenses (id, title, store, description, comments, reference_amount, booking_start, booking_end) VALUES
    ('00000000-0000-7000-8000-0000000000e1', 'e1', '', '', '', NULL,  '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000e2', 'e2', '', '', '', NULL,  '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000e3', 'e3', '', '', '', -2000, '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000e4', 'e4', '', '', '', NULL,  '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000e5', 'e5', '', '', '', NULL,  '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000e6', 'e6', '', '', '', NULL,  '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z'),
    ('00000000-0000-7000-8000-0000000000e7', 'e7', '', '', '', NULL,  '2025-06-01T12:00:00Z', '2025-06-01T12:00:00Z');

INSERT INTO expense_transactions (expense_id, account_id, idx, date, amount, fraction, comments, statement) VALUES
    ('00000000-0000-7000-8000-0000000000e1', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-01T12:00:00Z', -1250, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e2', '00000000-0000-7000-8000-0000000000a2', 0, '2025-06-01T12:00:00Z', -1000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e3', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-01T12:00:00Z', NULL, 0.25, '', ''),
    ('00000000-0000-7000-8000-0000000000e4', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-01T12:00:00Z', -1000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e4', '00000000-0000-7000-8000-0000000000a1', 1, '2025-06-01T12:00:00Z',  -500, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e5', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-01T12:00:00Z', -1000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e5', '00000000-0000-7000-8000-0000000000a7', 1, '2025-06-01T12:00:00Z',  -400, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e6', '00000000-0000-7000-8000-0000000000a3', 0, '2025-06-01T12:00:00Z', -1000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000e7', '00000000-0000-7000-8000-0000000000a5', 0, '2025-06-01T12:00:00Z', -1000, NULL, '', '');

INSERT INTO balances (id, account_id, date, amount, comments) VALUES
    ('00000000-0000-7000-8000-0000000000b1', '00000000-0000-7000-8000-0000000000a1', '2025-06-01T12:00:00Z', 5000, ''),
    ('00000000-0000-7000-8000-0000000000b2', '00000000-0000-7000-8000-0000000000a3', '2025-06-01T12:00:00Z', 5000, '');

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

ROLLBACK;
