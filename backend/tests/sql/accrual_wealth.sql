-- Covers accrual_adjustment(): the correction that turns the value-date wealth curve into one
-- read on the accrual axis. What was paid for and not yet used is still wealth, what was used
-- and not yet paid for is already gone, and outside every open period the correction is zero --
-- which is what keeps the two curves one line wherever nothing is in flight.
BEGIN;

INSERT INTO users (id, name, full_name, password_hash) VALUES
    ('00000000-0000-7000-8000-000000000001', 'mine',   'Mine',   'x'),
    -- owns the far leg of a shared expense, so a missing account filter shows up as this
    -- user's prepayment appearing in the other's wealth
    ('00000000-0000-7000-8000-000000000002', 'theirs', 'Theirs', 'x'),
    -- the transfer lives on its own user, so its two legs do not have to be netted out of
    -- every assertion above it
    ('00000000-0000-7000-8000-000000000003', 'moved',  'Moved',  'x');

INSERT INTO accounts (id, user_id, name, description, kind, availability, risk, opened, ownership_fraction) VALUES
    ('00000000-0000-7000-8000-0000000000a1', '00000000-0000-7000-8000-000000000001', 'a1', '', 'debit', 'immediately', 'none', '2024-01-01Z', 1.0),
    ('00000000-0000-7000-8000-0000000000a9', '00000000-0000-7000-8000-000000000002', 'a9', '', 'debit', 'immediately', 'none', '2024-01-01Z', 1.0),
    ('00000000-0000-7000-8000-0000000000b1', '00000000-0000-7000-8000-000000000003', 'b1', '', 'debit', 'immediately', 'none', '2024-01-01Z', 1.0),
    ('00000000-0000-7000-8000-0000000000b2', '00000000-0000-7000-8000-000000000003', 'b2', '', 'debit', 'immediately', 'none', '2024-01-01Z', 1.0);

INSERT INTO expenses (id, title, store, description, comments, reference_amount, accrual_start, accrual_end) VALUES
    -- Paid up front for the year that follows: 365 days at 100 cents a day, so a wrong
    -- denominator or a sample counted from the wrong end is a round number out.
    ('00000000-0000-7000-8000-0000000000f1', 'voraus',  '', '', '', NULL, '2025-01-01Z', '2026-01-01Z'),
    -- The same period, but the money is on two accounts belonging to two users. Only a1's leg
    -- is this user's wealth; a9's is the other user's, at ten times the size so a leak is not
    -- a rounding difference.
    ('00000000-0000-7000-8000-0000000000f2', 'geteilt', '', '', '', NULL, '2025-01-01Z', '2026-01-01Z'),
    -- Billed afterwards: 181 days at 100 cents a day, consumed from January and not paid
    -- until August.
    ('00000000-0000-7000-8000-0000000000f3', 'nachher', '', '', '', NULL, '2027-01-01Z', '2027-07-01Z'),
    -- an ordinary expense, consumed the day it was paid: it has to cancel itself at every
    -- sample rather than move the curve anywhere
    ('00000000-0000-7000-8000-0000000000f4', 'punkt',   '', '', '', NULL, '2026-06-01Z', '2026-06-01Z'),
    -- a point expense whose day is not the day it was paid, which happens often enough to matter.
    -- Between the two dates the money is gone and the value is not yet had.
    ('00000000-0000-7000-8000-0000000000f5', 'vorher',  '', '', '', NULL, '2026-04-01Z', '2026-04-01Z'),
    -- a transfer: two legs, four days apart, one instant of accrual for both
    ('00000000-0000-7000-8000-0000000000f6', 'umbuchung', '', '', '', NULL, '2025-03-01Z', '2025-03-01Z');

-- A rule's stanza over the same period as f1, carrying a real transaction on a real account.
-- Nothing but the flag says it never happened, and at ~2.7x f1 it cannot hide in a total.
INSERT INTO expenses (id, title, store, description, comments, reference_amount, accrual_start, accrual_end, is_template) VALUES
    ('00000000-0000-7000-8000-0000000000ff', 'template', '', '', '', NULL, '2025-01-01Z', '2026-01-01Z', true);

INSERT INTO expense_transactions (expense_id, account_id, idx, date, amount, fraction, comments, statement) VALUES
    ('00000000-0000-7000-8000-0000000000f1', '00000000-0000-7000-8000-0000000000a1', 0, '2025-01-01Z', -36500, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000f2', '00000000-0000-7000-8000-0000000000a1', 0, '2025-01-01Z',  -3650, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000f2', '00000000-0000-7000-8000-0000000000a9', 1, '2025-01-01Z', -36500, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000f3', '00000000-0000-7000-8000-0000000000a1', 0, '2027-08-01Z', -18100, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000f4', '00000000-0000-7000-8000-0000000000a1', 0, '2026-06-01Z',  -1234, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000f5', '00000000-0000-7000-8000-0000000000a1', 0, '2026-03-01Z',  -1000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000f6', '00000000-0000-7000-8000-0000000000b1', 0, '2025-03-01Z',  -5000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000f6', '00000000-0000-7000-8000-0000000000b2', 1, '2025-03-05Z',   5000, NULL, '', ''),
    ('00000000-0000-7000-8000-0000000000ff', '00000000-0000-7000-8000-0000000000a1', 0, '2025-01-01Z', -99999, NULL, '', '');

DO $$
DECLARE
    r record;
    failures int := 0;
    mine   uuid := '00000000-0000-7000-8000-000000000001';
    theirs uuid := '00000000-0000-7000-8000-000000000002';
    moved  uuid := '00000000-0000-7000-8000-000000000003';
BEGIN
    -- The correction is a fraction of a period, so every check below rounds: `extract(epoch)`
    -- is numeric, the division is not exact in it, and comparing 18300 against
    -- 18299,999999999999998 would fail on nothing.
    FOR r IN
        WITH checks(label, actual, expected) AS (
            -- Nothing has moved and nothing has been consumed at the instant it all starts:
            -- a movement is counted from the instant after its own, on both sides of the
            -- subtraction, or the correction would not cancel for an ordinary expense.
            SELECT 'the instant it is paid, neither axis has moved',
                (SELECT round(sum(a.amount)::numeric, 6) FROM accrual_adjustment(mine, ARRAY['2025-01-01T00:00:00Z'::timestamptz]) a), 0.0

            -- 182 of f1's 365 days are behind, so 18200 of the 36500 has been used and 18300
            -- is still owned; f2's leg on a1 adds a tenth of the same shape. A template
            -- counted here would add 50136,10 and the other user's leg another 18300,00.
            UNION ALL SELECT 'a prepayment is still wealth until it is used up',
                (SELECT round(sum(a.amount)::numeric, 6) FROM accrual_adjustment(mine, ARRAY['2025-07-02T00:00:00Z'::timestamptz]) a), 20130.0
            -- the far leg of the shared expense is the other user's, and is exactly what the
            -- assertion above must not contain
            UNION ALL SELECT 'the far leg belongs to the account owner',
                (SELECT round(sum(a.amount)::numeric, 6) FROM accrual_adjustment(theirs, ARRAY['2025-07-02T00:00:00Z'::timestamptz]) a), 18300.0

            -- 90 of f3's 181 days are behind and the bill does not arrive until August, so
            -- 9000 has been spent that no account has yet noticed
            UNION ALL SELECT 'a cost incurred before it is billed is already gone',
                (SELECT round(sum(a.amount)::numeric, 6) FROM accrual_adjustment(mine, ARRAY['2027-04-01T00:00:00Z'::timestamptz]) a), -9000.0
            -- and once it is paid the two axes agree about it again
            UNION ALL SELECT 'and stops correcting once the money follows',
                (SELECT round(sum(a.amount)::numeric, 6) FROM accrual_adjustment(mine, ARRAY['2027-09-01T00:00:00Z'::timestamptz]) a), 0.0

            -- f5 was paid on the 1st of March for the 1st of April: for that month the money
            -- is gone and the value has not been had, so it is still wealth
            UNION ALL SELECT 'money paid ahead of a point expense is still wealth',
                (SELECT round(sum(a.amount)::numeric, 6) FROM accrual_adjustment(mine, ARRAY['2026-03-15T00:00:00Z'::timestamptz]) a), 1000.0
            -- Both sides of the subtraction change at the instant *after* the event, never on
            -- it -- the same convention that has a recorded balance stated before the
            -- movements of its own instant. On f5's payment date nothing has happened yet,
            -- and on its accrual date the money has gone but the value has not yet arrived.
            -- The two have to agree: an ordinary expense, whose dates coincide, cancels only
            -- because they do.
            -- The second sample is there to hold f5 past the bounds filter, which drops a row
            -- lying entirely at or above every sample precisely because the convention is what
            -- it is. Asked at that instant alone the row never reaches the arithmetic, and the
            -- check would pass without exercising it.
            UNION ALL SELECT 'nothing has happened at the instant the money leaves',
                (SELECT round(a.amount::numeric, 6) FROM accrual_adjustment(mine,
                    ARRAY['2026-03-01T00:00:00Z', '2026-05-01T00:00:00Z']::timestamptz[]) a
                 WHERE a.sample_idx = 1), 0.0
            UNION ALL SELECT 'and the value has not arrived at the instant it is due',
                (SELECT round(sum(a.amount)::numeric, 6) FROM accrual_adjustment(mine, ARRAY['2026-04-01T00:00:00Z'::timestamptz]) a), 1000.0
            UNION ALL SELECT 'an ordinary expense cancels on its own instant too',
                (SELECT round(sum(a.amount)::numeric, 6) FROM accrual_adjustment(mine, ARRAY['2026-06-01T00:00:00Z'::timestamptz]) a), 0.0

            -- past every period and every payment: f4 in particular has to contribute nothing
            -- at any sample, having been consumed the day it was paid
            UNION ALL SELECT 'with nothing in flight the correction is exactly zero',
                (SELECT round(sum(a.amount)::numeric, 6) FROM accrual_adjustment(mine, ARRAY['2026-05-01T00:00:00Z'::timestamptz]) a), 0.0

            -- The value axis has the money out of b1 and not yet in b2. Both legs accrue at
            -- one instant, so the accrual axis considers it arrived and hands the amount back
            -- -- a transfer is not a cost, whichever day either bank posts it.
            UNION ALL SELECT 'money in flight is not a cost',
                (SELECT round(sum(a.amount)::numeric, 6) FROM accrual_adjustment(moved, ARRAY['2025-03-03T00:00:00Z'::timestamptz]) a), 5000.0
            UNION ALL SELECT 'and once it lands there is nothing to correct',
                (SELECT round(sum(a.amount)::numeric, 6) FROM accrual_adjustment(moved, ARRAY['2025-03-06T00:00:00Z'::timestamptz]) a), 0.0

            -- one row per sample whether or not anything is in flight at it, since the caller
            -- adds these onto a series and a missing row would shift the rest of it
            UNION ALL SELECT 'every sample is answered exactly once',
                (SELECT count(*) FROM accrual_adjustment(mine,
                    ARRAY['2025-07-02T00:00:00Z', '2026-05-01T00:00:00Z', '2027-04-01T00:00:00Z']::timestamptz[])), 3
            -- the ordinality is the caller's array index, which is what lets the corrections be
            -- lined up against the simulated totals without matching on dates again
            UNION ALL SELECT 'and keeps the index it was asked at',
                (SELECT round(a.amount::numeric, 6) FROM accrual_adjustment(mine,
                    ARRAY['2026-05-01T00:00:00Z', '2027-04-01T00:00:00Z']::timestamptz[]) a
                 WHERE a.sample_idx = 2), -9000.0
        )
        SELECT * FROM checks WHERE actual IS DISTINCT FROM expected
    LOOP
        RAISE WARNING 'FAIL % : got % want %', r.label, r.actual, r.expected;
        failures := failures + 1;
    END LOOP;

    IF failures > 0 THEN
        RAISE EXCEPTION '% accrual wealth check(s) failed', failures;
    END IF;
    RAISE NOTICE 'all accrual wealth checks passed';
END $$;

ROLLBACK;
