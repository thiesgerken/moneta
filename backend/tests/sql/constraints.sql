-- Every constraint that took over a check from Expense::validate or AccountInput::validate
-- has to actually reject, otherwise moving the check into the database bought nothing.
--
-- Each statement below is expected to fail. The EXCEPTION handler makes every attempt its own
-- subtransaction, so a rejection rolls itself back and the rest of the file carries on; a
-- statement that unexpectedly succeeds is collected and reported at the end.
BEGIN;

INSERT INTO users (id, name, full_name, password_hash)
VALUES ('00000000-0000-7000-8000-000000000001', 'u', 'U', 'x');
INSERT INTO accounts (id, user_id, name, description, kind, availability, risk, opened, ownership_fraction) VALUES
    ('00000000-0000-7000-8000-0000000000a1', '00000000-0000-7000-8000-000000000001', 'a1', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0),
    ('00000000-0000-7000-8000-0000000000a2', '00000000-0000-7000-8000-000000000001', 'a2', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.0);
INSERT INTO categories (id, name, description) VALUES ('00000000-0000-7000-8000-0000000000c1', 'c', '');
INSERT INTO expenses (id, title, store, description, comments, accrual_start, accrual_end)
VALUES ('00000000-0000-7000-8000-0000000000e1', 'e', '', '', '', '2025-06-01Z', '2025-06-01Z');

DO $$
DECLARE
    c record;
    slipped text[] := '{}';
    checked int := 0;
BEGIN
    FOR c IN SELECT * FROM (VALUES
        ('backwards booking period',
         $q$INSERT INTO expenses (title, store, description, comments, accrual_start, accrual_end)
            VALUES ('bad', '', '', '', '2025-12-31Z', '2025-01-01Z')$q$),

        ('transaction with both amount and fraction',
         $q$INSERT INTO expense_transactions (expense_id, account_id, idx, date, amount, fraction, comments, statement)
            VALUES ('00000000-0000-7000-8000-0000000000e1', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-01Z', -100, 0.5, '', '')$q$),

        ('transaction with neither amount nor fraction',
         $q$INSERT INTO expense_transactions (expense_id, account_id, idx, date, amount, fraction, comments, statement)
            VALUES ('00000000-0000-7000-8000-0000000000e1', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-01Z', NULL, NULL, '', '')$q$),

        ('transaction with a zero fraction',
         $q$INSERT INTO expense_transactions (expense_id, account_id, idx, date, amount, fraction, comments, statement)
            VALUES ('00000000-0000-7000-8000-0000000000e1', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-01Z', NULL, 0.0, '', '')$q$),

        ('duplicate transaction idx within one expense',
         $q$INSERT INTO expense_transactions (expense_id, account_id, idx, date, amount, comments, statement) VALUES
            ('00000000-0000-7000-8000-0000000000e1', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-01Z', -100, '', ''),
            ('00000000-0000-7000-8000-0000000000e1', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-01Z', -200, '', '')$q$),

        ('transaction pointing at no expense',
         $q$INSERT INTO expense_transactions (expense_id, account_id, idx, date, amount, comments, statement)
            VALUES ('00000000-0000-7000-8000-00000000dead', '00000000-0000-7000-8000-0000000000a1', 0, '2025-06-01Z', -100, '', '')$q$),

        ('categorization with a zero weight',
         $q$INSERT INTO expense_categorizations (expense_id, category_id, weight)
            VALUES ('00000000-0000-7000-8000-0000000000e1', '00000000-0000-7000-8000-0000000000c1', 0.0)$q$),

        ('categorization with a negative weight',
         $q$INSERT INTO expense_categorizations (expense_id, category_id, weight)
            VALUES ('00000000-0000-7000-8000-0000000000e1', '00000000-0000-7000-8000-0000000000c1', -1.0)$q$),

        ('the same category twice on one expense',
         $q$INSERT INTO expense_categorizations (expense_id, category_id, weight) VALUES
            ('00000000-0000-7000-8000-0000000000e1', '00000000-0000-7000-8000-0000000000c1', 1.0),
            ('00000000-0000-7000-8000-0000000000e1', '00000000-0000-7000-8000-0000000000c1', 2.0)$q$),

        ('ownership fraction above 1',
         $q$INSERT INTO accounts (user_id, name, description, kind, availability, risk, opened, ownership_fraction)
            VALUES ('00000000-0000-7000-8000-000000000001', 'a', '', 'debit', 'immediately', 'none', '2025-01-01Z', 1.5)$q$),

        ('ownership fraction of zero',
         $q$INSERT INTO accounts (user_id, name, description, kind, availability, risk, opened, ownership_fraction)
            VALUES ('00000000-0000-7000-8000-000000000001', 'a', '', 'debit', 'immediately', 'none', '2025-01-01Z', 0.0)$q$),

        ('unknown account kind',
         $q$INSERT INTO accounts (user_id, name, description, kind, availability, risk, opened, ownership_fraction)
            VALUES ('00000000-0000-7000-8000-000000000001', 'a', '', 'bitcoin', 'immediately', 'none', '2025-01-01Z', 1.0)$q$),

        ('unknown availability',
         $q$INSERT INTO accounts (user_id, name, description, kind, availability, risk, opened, ownership_fraction)
            VALUES ('00000000-0000-7000-8000-000000000001', 'a', '', 'debit', 'eventually', 'none', '2025-01-01Z', 1.0)$q$),

        ('account closed before it opened',
         $q$INSERT INTO accounts (user_id, name, description, kind, availability, risk, opened, closed, ownership_fraction)
            VALUES ('00000000-0000-7000-8000-000000000001', 'a', '', 'debit', 'immediately', 'none', '2025-06-01Z', '2025-01-01Z', 1.0)$q$),

        ('sync_account_id without sync_invert',
         $q$UPDATE accounts SET sync_account_id = '00000000-0000-7000-8000-0000000000a2'
            WHERE id = '00000000-0000-7000-8000-0000000000a1'$q$),

        ('sync_invert without sync_account_id',
         $q$UPDATE accounts SET sync_invert = true WHERE id = '00000000-0000-7000-8000-0000000000a1'$q$),

        ('account synchronized to itself',
         $q$UPDATE accounts SET sync_account_id = id, sync_invert = false
            WHERE id = '00000000-0000-7000-8000-0000000000a1'$q$),

        ('category as its own parent',
         $q$UPDATE categories SET parent_id = id WHERE id = '00000000-0000-7000-8000-0000000000c1'$q$),

        ('changelog entry with neither state',
         $q$INSERT INTO expense_changes (target_id, date, automatic, confidence, old_state, new_state)
            VALUES ('00000000-0000-7000-8000-0000000000e1', '2025-06-01Z', false, 1.0, NULL, NULL)$q$),

        ('changelog confidence above 1',
         $q$INSERT INTO expense_changes (target_id, date, automatic, confidence, new_state)
            VALUES ('00000000-0000-7000-8000-0000000000e1', '2025-06-01Z', true, 1.5, '{}')$q$),

        ('unknown periodic interval',
         $q$INSERT INTO periodic_rules (user_id, title, template_id, period, last_creation)
            VALUES ('00000000-0000-7000-8000-000000000001', 'r', '00000000-0000-7000-8000-0000000000e1', 'fortnightly', '2025-01-01Z')$q$),

        ('the same delivery seen twice',
         $q$INSERT INTO seen_deliveries (user_id, account_id, date, amount, statement) VALUES
            ('00000000-0000-7000-8000-000000000001', '00000000-0000-7000-8000-0000000000a1', '2025-06-01Z', -100, 'REWE'),
            ('00000000-0000-7000-8000-000000000001', '00000000-0000-7000-8000-0000000000a1', '2025-06-01Z', -100, 'REWE')$q$),

        ('receipts that are not a json array',
         $q$INSERT INTO expenses (title, store, description, comments, accrual_start, accrual_end, receipts)
            VALUES ('bad', '', '', '', '2025-06-01Z', '2025-06-01Z', '{"uri": "x"}')$q$),

        ('positions that are not a json array',
         $q$INSERT INTO expenses (title, store, description, comments, accrual_start, accrual_end, positions)
            VALUES ('bad', '', '', '', '2025-06-01Z', '2025-06-01Z', '"nope"')$q$),

        ('monthly limit of zero',
         $q$INSERT INTO categories (name, description, monthly_limit) VALUES ('c', '', 0)$q$)
    ) AS t(label, stmt)
    LOOP
        checked := checked + 1;
        BEGIN
            EXECUTE c.stmt;
            slipped := slipped || c.label;
        EXCEPTION
            WHEN check_violation OR unique_violation OR foreign_key_violation
                OR restrict_violation THEN NULL;
        END;
    END LOOP;

    IF array_length(slipped, 1) > 0 THEN
        RAISE EXCEPTION 'not rejected: %', array_to_string(slipped, '; ');
    END IF;
    RAISE NOTICE '% constraint checks rejected as expected', checked;
END $$;

-- Deleting an account another one mirrors has to be refused rather than silently unlinked:
-- ON DELETE SET NULL would leave sync_invert dangling.
DO $$
BEGIN
    UPDATE accounts SET sync_account_id = '00000000-0000-7000-8000-0000000000a2', sync_invert = true
    WHERE id = '00000000-0000-7000-8000-0000000000a1';

    BEGIN
        DELETE FROM accounts WHERE id = '00000000-0000-7000-8000-0000000000a2';
        RAISE EXCEPTION 'deleting a mirrored account was allowed';
    EXCEPTION
        -- ON DELETE RESTRICT raises restrict_violation (23001), not foreign_key_violation
        WHEN restrict_violation THEN NULL;
    END;
END $$;

-- ... while the shapes the app relies on must still go in.
DO $$
BEGIN
    -- an ordinary expense can have a single-instant accrual period
    INSERT INTO expenses (title, store, description, comments, accrual_start, accrual_end)
    VALUES ('ok', '', '', '', '2025-06-01Z', '2025-06-01Z');

    -- a prepayment consumed over a year
    INSERT INTO expenses (title, store, description, comments, accrual_start, accrual_end)
    VALUES ('ok', '', '', '', '2025-01-01Z', '2025-12-31Z');

    RAISE NOTICE 'valid rows still accepted';
END $$;

ROLLBACK;
