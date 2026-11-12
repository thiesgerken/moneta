//! Expense mutations through the real GraphQL handler, against disposable PostgreSQL 18.
//!
//! Needs an empty database of its own, handed in as `TEST_DATABASE_URL`. It applies the
//! migrations and truncates as it goes, so run it after the other suites, not alongside them.

use chrono::{DateTime, TimeZone, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use rocket::{http::ContentType, local::asynchronous::Client};
use serde_json::{Value, json};
use uuid::Uuid;

use moneta::graphql::cache::{AnalysisCache, cached};
use moneta::models::{
    Account, AccountAvailability, AccountKind, AccountRisk, Expense, ExpenseTransaction, User,
};
use moneta::schema::{accounts, expense_changes, users};

const MINE: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_0001);
const THEIRS: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_0002);
const MY_ACCOUNT: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_00a1);
const THEIR_ACCOUNT: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_00a2);
const MY_UNCHECKED: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_00e1);
const MY_CHECKED: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_00e2);
const THEIR_UNCHECKED: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_00e3);

const CONFIRM: &str =
    "mutation($ids: [UUID!]!) { expenses { confirm(ids: $ids) { id isUnchecked } } }";

fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, 12, 0, 0).unwrap()
}

fn account(id: Uuid, user_id: Uuid) -> Account {
    Account {
        id,
        user_id,
        name: "Giro".into(),
        description: String::new(),
        color: None,
        iban: None,
        kind: AccountKind::Debit,
        availability: AccountAvailability::Immediately,
        risk: AccountRisk::None,
        opened: at(2030, 1, 1),
        closed: None,
        ownership_fraction: 1.0,
        sync_account_id: None,
        sync_invert: None,
        renditier_account_id: None,
    }
}

fn expense(id: Uuid, account_id: Uuid, is_unchecked: bool) -> Expense {
    Expense {
        id,
        title: "REWE".into(),
        store: String::new(),
        description: String::new(),
        comments: String::new(),
        reference_amount: None,
        accrual_start: at(2030, 6, 1),
        accrual_end: at(2030, 6, 1),
        is_template: false,
        is_tax_relevant: false,
        is_unchecked,
        is_incomplete: false,
        is_extraordinary: false,
        receipts: Vec::new(),
        categories: Vec::new(),
        transactions: vec![ExpenseTransaction {
            account_id,
            date: at(2030, 6, 1),
            booking_date: None,
            amount: Some(-1000),
            fraction: None,
            comments: String::new(),
            statement: String::new(),
        }],
        positions: Vec::new(),
        created_at: at(2030, 6, 1),
        updated_at: at(2030, 6, 1),
    }
}

async fn fixtures(conn: &mut AsyncPgConnection) {
    diesel::sql_query(
        "TRUNCATE expense_changes, seen_deliveries, automation_rules, periodic_rules, \
         expense_categorizations, expense_transactions, expenses, balances, accounts, \
         categories, users CASCADE",
    )
    .execute(conn)
    .await
    .expect("could not clear the test database");

    let password_hash = Some(moneta::hash_password("dummy-password"));
    diesel::insert_into(users::table)
        .values(vec![
            User {
                id: MINE,
                name: "dummy-alice".into(),
                full_name: "dummy-alice".into(),
                password_hash: password_hash.clone(),
            },
            User {
                id: THEIRS,
                name: "dummy-bob".into(),
                full_name: "dummy-bob".into(),
                password_hash,
            },
        ])
        .execute(conn)
        .await
        .expect("users");

    diesel::insert_into(accounts::table)
        .values(vec![
            account(MY_ACCOUNT, MINE),
            account(THEIR_ACCOUNT, THEIRS),
        ])
        .execute(conn)
        .await
        .expect("accounts");

    for e in [
        expense(MY_UNCHECKED, MY_ACCOUNT, true),
        expense(MY_CHECKED, MY_ACCOUNT, false),
        expense(THEIR_UNCHECKED, THEIR_ACCOUNT, true),
    ] {
        moneta::queries::insert_expense(conn, &e)
            .await
            .expect("expense");
    }
}

async fn post(client: &Client, query: &str, variables: Value) -> Value {
    client
        .post("/graphql")
        .header(ContentType::JSON)
        .body(json!({ "query": query, "variables": variables }).to_string())
        .dispatch()
        .await
        .into_json()
        .await
        .expect("JSON response")
}

async fn is_unchecked(pool: &moneta::Pool, id: Uuid) -> bool {
    let conn = &mut pool.get().await.unwrap();
    moneta::queries::load_expense(conn, id)
        .await
        .unwrap()
        .expect("expense")
        .is_unchecked
}

async fn changes(pool: &moneta::Pool) -> Vec<(Uuid, Value, Value)> {
    expense_changes::table
        .select((
            expense_changes::target_id,
            expense_changes::old_state,
            expense_changes::new_state,
        ))
        .load::<(Uuid, Option<Value>, Option<Value>)>(&mut pool.get().await.unwrap())
        .await
        .unwrap()
        .into_iter()
        .map(|(target, old, new)| (target, old.expect("old"), new.expect("new")))
        .collect()
}

/// One test rather than several: they share one database, and a second test truncating it
/// while the first is reading would be a race with a very confusing failure.
#[tokio::test]
async fn confirm_clears_the_flag_on_what_the_reader_can_see() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        assert!(
            std::env::var("MONETA_REQUIRE_DB_TESTS").is_err(),
            "TEST_DATABASE_URL is required"
        );
        eprintln!("TEST_DATABASE_URL is not set -- skipping the expense mutation tests");
        return;
    };

    moneta::migrate(&url).await.expect("migrations");
    let pool = moneta::pool(&url, "moneta-expense-test").expect("pool");
    fixtures(&mut pool.get().await.unwrap()).await;

    let config = moneta::config::Config {
        verbosity: -2,
        ..Default::default()
    };
    let client = Client::tracked(moneta::graphql::rocket(config, pool.clone()))
        .await
        .unwrap();
    let login = post(
        &client,
        "mutation { users { login(name: \"dummy-alice\", password: \"dummy-password\") { id } } }",
        Value::Null,
    )
    .await;
    assert!(login.get("errors").is_none(), "{login}");

    // somebody else's expense among the ids fails the whole request, including the part that
    // would have been allowed
    let refused = post(
        &client,
        CONFIRM,
        json!({ "ids": [MY_UNCHECKED, THEIR_UNCHECKED] }),
    )
    .await;
    assert!(refused["errors"].is_array(), "{refused}");
    assert!(is_unchecked(&pool, MY_UNCHECKED).await);
    assert!(is_unchecked(&pool, THEIR_UNCHECKED).await);
    assert!(changes(&pool).await.is_empty());

    let cache = client
        .rocket()
        .state::<std::sync::Arc<AnalysisCache>>()
        .unwrap();
    cached(cache, "stale".into(), async { Ok(1) })
        .await
        .unwrap();

    let confirmed = post(
        &client,
        CONFIRM,
        json!({ "ids": [MY_UNCHECKED, MY_CHECKED] }),
    )
    .await;
    assert!(confirmed.get("errors").is_none(), "{confirmed}");
    assert_eq!(
        confirmed["data"]["expenses"]["confirm"],
        json!([
            { "id": MY_UNCHECKED, "isUnchecked": false },
            { "id": MY_CHECKED, "isUnchecked": false },
        ])
    );
    assert!(!is_unchecked(&pool, MY_UNCHECKED).await);
    assert!(is_unchecked(&pool, THEIR_UNCHECKED).await);

    // one entry, for the expense that changed; the one already confirmed gets none
    let changes = changes(&pool).await;
    assert_eq!(changes.len(), 1, "{changes:?}");
    let (target, old, new) = &changes[0];
    assert_eq!(*target, MY_UNCHECKED);
    assert_eq!(old["isUnchecked"], true);
    assert_eq!(new["isUnchecked"], false);

    assert_eq!(
        cached(cache, "stale".into(), async { Ok(2) })
            .await
            .unwrap(),
        2,
        "a confirmation has to empty the analysis cache"
    );
}
