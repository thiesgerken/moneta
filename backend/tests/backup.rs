//! `moneta export` and `moneta import` against a real database: a round trip that has to give
//! back every row as it was, and the refusals that keep an import from writing half a backup.
//!
//! The fixtures are plain SQL rather than a backup restored through the code under test, so a
//! column that both halves forget cannot come back looking right.
//!
//! Needs an empty database of its own, handed in as `TEST_DATABASE_URL`. It applies the
//! migrations and truncates as it goes, so do not point it at anything you want to keep.

use std::collections::BTreeSet;

use diesel::sql_types::{Jsonb, Text};
use diesel_async::{AsyncPgConnection, RunQueryDsl, SimpleAsyncConnection};
use serde_json::Value;

use moneta::backup::Backup;

/// Every table except the ones holding credentials, with the order a comparison reads it in.
const TABLES: [(&str, &str); 11] = [
    ("users", "id"),
    ("categories", "id"),
    ("accounts", "id"),
    ("expenses", "id"),
    ("expense_transactions", "expense_id, idx"),
    ("expense_categorizations", "expense_id, category_id"),
    ("balances", "id"),
    ("periodic_rules", "id"),
    ("automation_rules", "id"),
    ("seen_deliveries", "id"),
    ("expense_changes", "id"),
];

/// Left out of the comparison: credentials are not exported, and a transaction's id is not
/// part of the expense aggregate (every edit rewrites it anyway).
const NOT_COMPARED: [(&str, &str); 3] = [
    ("users", "password_hash"),
    ("users", "renditier_token"),
    ("expense_transactions", "id"),
];

const CLEAR: &str = "TRUNCATE users, categories, accounts, expenses, expense_transactions, \
    expense_categorizations, balances, periodic_rules, automation_rules, seen_deliveries, \
    expense_changes, api_keys CASCADE";

/// Two users sharing a debt, with every column set to something other than its default
/// somewhere. All of it invented.
const FIXTURES: &str = r#"
INSERT INTO users (id, name, full_name, password_hash, seen_at, seen_before, renditier_token) VALUES
    ('00000000-0000-7000-8000-000000000001', 'rudolph', 'Rudolph Rentier',
     '$argon2id$v=19$m=19456,t=2,p=1$aW52ZW50ZWQ$aW52ZW50ZWRoYXNo', '2030-03-02 08:00+00',
     '2030-03-01 08:00+00', 'invented-renditier-token'),
    ('00000000-0000-7000-8000-000000000002', 'kroesus', 'Krösus',
     '$argon2id$v=19$m=19456,t=2,p=1$aW52ZW50ZWQ$aW52ZW50ZWRoYXNo', NULL, NULL, NULL);

INSERT INTO api_keys (user_id, name, prefix, secret_hash, access_level) VALUES
    ('00000000-0000-7000-8000-000000000001', 'Ferret', '0123abcd',
     decode(repeat('ab', 32), 'hex'), 'read');

INSERT INTO categories (id, name, description, color, parent_id, monthly_limit) VALUES
    ('00000000-0000-7000-8000-0000000000c2', 'Lebensmittel', 'Supermarkt', '#00aa00',
     '00000000-0000-7000-8000-0000000000c1', 40000),
    ('00000000-0000-7000-8000-0000000000c1', 'Haushalt', '', NULL, NULL, NULL);

INSERT INTO accounts (id, user_id, name, description, color, iban, kind, availability, risk,
                      opened, closed, ownership_fraction, sync_account_id, sync_invert,
                      renditier_account_id) VALUES
    ('00000000-0000-7000-8000-0000000000a1', '00000000-0000-7000-8000-000000000001',
     'Girokonto', 'Commerzbank', '#ffcc00', 'DE00000000000000000000', 'debit', 'immediately',
     'none', '2030-01-01 00:00+00', NULL, 1, NULL, NULL, NULL),
    ('00000000-0000-7000-8000-0000000000a2', '00000000-0000-7000-8000-000000000001',
     'Depot', '', NULL, NULL, 'real_estate', 'decades', 'huge', '2030-01-01 00:00+00',
     '2032-12-31 00:00+00', 0.3333333333333333, NULL, NULL, 7),
    ('00000000-0000-7000-8000-0000000000a3', '00000000-0000-7000-8000-000000000001',
     'Schulden bei Krösus', '', NULL, NULL, 'debt', 'days', 'slight', '2030-01-01 00:00+00',
     NULL, 1, '00000000-0000-7000-8000-0000000000a4', true, NULL),
    ('00000000-0000-7000-8000-0000000000a4', '00000000-0000-7000-8000-000000000002',
     'Forderung an Rudolph', '', NULL, NULL, 'debt', 'days', 'slight', '2030-01-01 00:00+00',
     NULL, 1, '00000000-0000-7000-8000-0000000000a3', true, NULL),
    ('00000000-0000-7000-8000-0000000000a5', '00000000-0000-7000-8000-000000000002',
     'Bargeld', '', NULL, NULL, 'cash', 'immediately', 'low', '2030-01-01 00:00+00',
     NULL, 1, NULL, NULL, NULL);

INSERT INTO expenses (id, title, store, description, comments, reference_amount, accrual_start,
                      accrual_end, is_template, is_tax_relevant, is_unchecked, is_incomplete,
                      is_extraordinary, receipts, positions, created_at, updated_at) VALUES
    ('00000000-0000-7000-8000-0000000000e1', 'Wocheneinkauf', 'REWE', 'geteilt', 'halbe-halbe',
     -9000, '2030-02-01 12:00+00', '2030-02-01 12:00+00', false, true, true, false, false,
     '[{"uri": "dms://beleg/1", "ocr": "REWE SAGT DANKE"}]',
     '[{"amount": -450, "quantity": 1.5, "unit": "kg", "title": "Äpfel", "comments": ""}]',
     '2030-02-01 13:00+00', '2030-02-02 09:30:00.123456+00'),
    ('00000000-0000-7000-8000-0000000000e2', 'Bargeld', 'BEISPIELMARKT/TESTSTADT', '', '',
     NULL, '2030-01-01 12:00+00', '2030-12-31 12:00+00', true, false, false, true, true,
     '[]', '[]', '2030-01-01 13:00+00', '2030-01-01 13:00+00');

INSERT INTO expense_transactions (expense_id, account_id, idx, date, amount, fraction, comments,
                                  statement, booking_date) VALUES
    ('00000000-0000-7000-8000-0000000000e1', '00000000-0000-7000-8000-0000000000a1', 0,
     '2030-02-01 12:00+00', NULL, 1, '', 'REWE SAGT DANKE', '2030-02-03 00:00+00'),
    ('00000000-0000-7000-8000-0000000000e1', '00000000-0000-7000-8000-0000000000a3', 1,
     '2030-02-01 12:00+00', NULL, -0.3333333333333333, 'Krösus'' Anteil', '', NULL),
    ('00000000-0000-7000-8000-0000000000e1', '00000000-0000-7000-8000-0000000000a5', 2,
     '2030-02-01 12:00+00', -1, NULL, 'Pfand', '', NULL),
    ('00000000-0000-7000-8000-0000000000e2', '00000000-0000-7000-8000-0000000000a5', 0,
     '2030-01-01 12:00+00', 0, NULL, '', '', NULL);

-- 1/11 is a double serde_json reads back one bit off unless float_roundtrip is on
INSERT INTO expense_categorizations (expense_id, category_id, weight) VALUES
    ('00000000-0000-7000-8000-0000000000e1', '00000000-0000-7000-8000-0000000000c2',
     0.09090909090909091),
    ('00000000-0000-7000-8000-0000000000e1', '00000000-0000-7000-8000-0000000000c1',
     0.9090909090909091);

INSERT INTO balances (id, account_id, date, amount, comments, created_at, updated_at) VALUES
    ('00000000-0000-7000-8000-0000000000b1', '00000000-0000-7000-8000-0000000000a1',
     '2030-03-01 00:00+00', 123456, 'Kontoauszug', '2030-03-01 10:00+00', '2030-03-01 11:00+00');

INSERT INTO periodic_rules (id, user_id, title, template_id, period, title_replacement,
                            last_creation, created_at, updated_at) VALUES
    ('00000000-0000-7000-8000-0000000000f1', '00000000-0000-7000-8000-000000000001', 'Taschengeld',
     '00000000-0000-7000-8000-0000000000e2', 'monthly', 'Taschengeld %m/%y',
     '2030-03-01 12:00+00', '2030-01-01 13:00+00', '2030-01-02 13:00+00');

INSERT INTO automation_rules (id, user_id, title, template_id, priority, regex_time,
                              regex_statement, filter_amount, filter_account_id, last_delivery,
                              created_at, updated_at) VALUES
    ('00000000-0000-7000-8000-0000000000f2', '00000000-0000-7000-8000-000000000002', 'Automat',
     '00000000-0000-7000-8000-0000000000e2', 5, '^01\.', 'BEISPIELMARKT', -5000,
     '00000000-0000-7000-8000-0000000000a5', '2030-02-01 12:00+00', '2030-01-01 13:00+00',
     '2030-01-02 13:00+00');

INSERT INTO seen_deliveries (id, user_id, account_id, date, amount, statement, expense_id,
                             created_at) VALUES
    ('00000000-0000-7000-8000-0000000000d1', '00000000-0000-7000-8000-000000000001',
     '00000000-0000-7000-8000-0000000000a1', '2030-02-01 12:00+00', -9000, 'REWE SAGT DANKE',
     '00000000-0000-7000-8000-0000000000e1', '2030-02-03 06:00+00');

-- the second snapshot carries a field today's Expense does not have, and the third entry
-- describes an expense that no longer exists, by somebody who no longer exists either
INSERT INTO expense_changes (id, target_id, user_id, date, tool, automatic, confidence,
                             old_state, new_state) VALUES
    ('00000000-0000-7000-8000-000000000ac1', '00000000-0000-7000-8000-0000000000e1',
     '00000000-0000-7000-8000-000000000001', '2030-02-01 13:00+00', 'web', false, 0, NULL,
     '{"id": "00000000-0000-7000-8000-0000000000e1", "title": "Einkauf", "amount": -90.1}'),
    ('00000000-0000-7000-8000-000000000ac2', '00000000-0000-7000-8000-0000000000e1',
     '00000000-0000-7000-8000-000000000002', '2030-02-02 09:30+00', NULL, false, 0,
     '{"id": "00000000-0000-7000-8000-0000000000e1", "title": "Einkauf", "amount": -90.1}',
     '{"id": "00000000-0000-7000-8000-0000000000e1", "title": "Wocheneinkauf", "bookingStart": "2030-02-01"}'),
    ('00000000-0000-7000-8000-000000000ac3', '00000000-0000-7000-8000-0000000000e9', NULL,
     '2030-02-05 10:00+00', 'cli', true, 0.875, '{"id": "00000000-0000-7000-8000-0000000000e9"}',
     NULL);
"#;

#[derive(diesel::QueryableByName)]
struct Rows {
    #[diesel(sql_type = Jsonb)]
    rows: Value,
}

#[derive(diesel::QueryableByName)]
struct Name {
    #[diesel(sql_type = Text)]
    name: String,
}

/// Every table as postgres sees it, minus what is not supposed to survive.
async fn tables(conn: &mut AsyncPgConnection) -> Vec<(&'static str, Value)> {
    let mut all = Vec::new();
    for (table, order) in TABLES {
        let minus: String = NOT_COMPARED
            .iter()
            .filter(|(t, _)| *t == table)
            .map(|(_, column)| format!(" - '{column}'"))
            .collect();
        let rows: Rows = diesel::sql_query(format!(
            "SELECT coalesce(jsonb_agg(to_jsonb(t){minus} ORDER BY {order}), '[]') AS rows \
             FROM {table} t"
        ))
        .get_result(conn)
        .await
        .expect("could not read a table");
        all.push((table, rows.rows));
    }
    all
}

async fn sql(conn: &mut AsyncPgConnection, statements: &str) {
    conn.batch_execute(statements)
        .await
        .unwrap_or_else(|e| panic!("{e}: {statements}"));
}

async fn count(conn: &mut AsyncPgConnection, table: &str) -> i64 {
    #[derive(diesel::QueryableByName)]
    struct Count {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        count: i64,
    }
    diesel::sql_query(format!("SELECT count(*) AS count FROM {table}"))
        .get_result::<Count>(conn)
        .await
        .expect("could not count")
        .count
}

fn export(backup: &Backup) -> String {
    serde_json::to_string_pretty(backup).expect("could not serialize the backup")
}

/// One test rather than several: they share one database, and a second test truncating it
/// while the first is reading would be a race with a very confusing failure.
#[tokio::test]
async fn export_and_import_round_trip_and_refuse() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        assert!(
            std::env::var("MONETA_REQUIRE_DB_TESTS").is_err(),
            "TEST_DATABASE_URL is required"
        );
        eprintln!("TEST_DATABASE_URL is not set -- skipping the database-backed backup tests");
        return;
    };

    moneta::migrate(&url).await.expect("migrations");
    let pool = moneta::pool(&url, "moneta-backup-test").expect("pool");
    let conn = &mut pool.get().await.expect("connection");

    // --- every table is either in the backup or deliberately not -------------------------

    let found: BTreeSet<String> = diesel::sql_query(
        "SELECT table_name::text AS name FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_type = 'BASE TABLE'",
    )
    .load::<Name>(conn)
    .await
    .expect("tables")
    .into_iter()
    .map(|n| n.name)
    .collect();
    let mut known: BTreeSet<String> = TABLES.iter().map(|(t, _)| t.to_string()).collect();
    known.extend(["api_keys".into(), "__diesel_schema_migrations".into()]);
    assert_eq!(
        found, known,
        "a table was added or removed: decide whether the backup carries it"
    );

    // --- round trip ------------------------------------------------------------------------

    sql(conn, CLEAR).await;
    sql(conn, FIXTURES).await;

    // a column nobody set cannot show that the export carries it
    for (table, rows) in tables(conn).await {
        let rows = rows.as_array().unwrap();
        let first = rows[0].as_object().unwrap();
        for column in first.keys() {
            assert!(
                rows.iter().any(|row| !row[column].is_null()),
                "{table}.{column} is null in every fixture row"
            );
        }
    }

    let before = tables(conn).await;
    let first = Backup::load(conn).await.expect("export");
    let exported = export(&first);

    for secret in ["argon2", "invented-renditier-token", "0123abcd"] {
        assert!(
            !exported.contains(secret),
            "the export carries a credential: {secret}"
        );
    }

    sql(conn, CLEAR).await;
    let parsed: Backup = serde_json::from_str(&exported).expect("the export parses");
    parsed
        .restore(conn)
        .await
        .expect("import into an empty database");

    assert_eq!(tables(conn).await, before, "the import changed a row");
    assert_eq!(
        export(&Backup::load(conn).await.expect("second export")),
        exported,
        "exporting the import gave a different document"
    );

    #[derive(diesel::QueryableByName)]
    struct Credentials {
        #[diesel(sql_type = Text)]
        password_hash: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
        renditier_token: Option<String>,
    }
    let credentials: Vec<Credentials> =
        diesel::sql_query("SELECT password_hash, renditier_token FROM users")
            .load(conn)
            .await
            .expect("users");
    assert!(
        credentials
            .iter()
            .all(|c| !moneta::verify_password("", &c.password_hash) && c.renditier_token.is_none()),
        "an imported user can sign in or reach renditier without being given a credential"
    );
    assert_eq!(count(conn, "api_keys").await, 0);

    // --- refusals --------------------------------------------------------------------------

    let error = parsed
        .restore(conn)
        .await
        .expect_err("an import over existing data has to be refused");
    assert!(
        error.to_string().contains("already holds user data"),
        "{error}"
    );

    // one user is enough to make a database not empty, and the refusal writes nothing
    sql(conn, CLEAR).await;
    sql(
        conn,
        "INSERT INTO users (name, full_name, password_hash) VALUES ('rudolph', 'Rudolph', 'x')",
    )
    .await;
    let error = parsed
        .restore(conn)
        .await
        .expect_err("an import next to an existing user has to be refused");
    assert!(
        error.to_string().contains("already holds user data"),
        "{error}"
    );
    assert_eq!(count(conn, "users").await, 1);
    assert_eq!(count(conn, "expenses").await, 0);

    // what only the database notices rolls back everything written before it
    sql(conn, CLEAR).await;
    let mut broken: Backup = serde_json::from_str(&exported).unwrap();
    broken.accounts[0].ownership_fraction = 2.0;
    let error = broken
        .restore(conn)
        .await
        .expect_err("a CHECK violation has to fail the import");
    assert!(
        error
            .to_string()
            .contains("accounts_ownership_fraction_range"),
        "{error}"
    );
    for (table, _) in TABLES {
        assert_eq!(
            count(conn, table).await,
            0,
            "{table} kept rows of a failed import"
        );
    }

    // an empty export is a valid one
    let empty = Backup::load(conn).await.expect("export of nothing");
    empty.restore(conn).await.expect("import of nothing");
    assert_eq!(export(&Backup::load(conn).await.unwrap()), export(&empty));
}
