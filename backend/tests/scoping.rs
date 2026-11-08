//! The per-user filters, against a real database.
//!
//! These test `graphql::scoped` rather than the resolvers, and that is the point: since
//! `no_resolver_reaches_a_scoped_table_directly` holds, every resolver read goes through one of
//! these accessors, so proving the accessors scope proves the whole surface. Six resolver
//! modules' worth of cases collapse into one file.
//!
//! Why a database and not a mock: what is being checked is whether the filter is *there and
//! right*, and that lives inside the query a mock would replace. A fake connection would report
//! green on the one thing nobody can afford to be wrong about.
//!
//! Why postgres and not sqlite: the rendered views are the thing under test, and they are built
//! from `CROSS JOIN LATERAL`, `jsonb_agg` and `uuidv7()`. In sqlite they cannot exist, so the
//! only way to run this there is against a schema that is not the one that ships.
//!
//! Needs an empty database of its own, handed in as `TEST_DATABASE_URL`. It applies the
//! migrations and truncates as it goes, so do not point it at anything you want to keep.

use chrono::{DateTime, TimeZone, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use moneta::graphql::scoped;
use moneta::models::rendering::{RenderedBalance, RenderedExpense};
use moneta::models::{
    Account, AccountAvailability, AccountKind, AccountRisk, Balance, Expense, ExpenseTransaction,
    User,
};
use moneta::schema::{accounts, balances, users};
use moneta::schema_views::{balances_rendered, expense_transactions_rendered, expenses_rendered};

/// Readable ids, so a failure names the row rather than a uuid nobody recognises.
const MINE: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_0001);
const THEIRS: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_0002);
const MY_ACCOUNT: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_00a1);
const THEIR_ACCOUNT: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_00a2);
const MY_MIRROR: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_00a3);
const THEIR_MIRROR: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_00a4);
const MY_EXPENSE: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_00e1);
const THEIR_EXPENSE: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_00e2);
const SHARED_EXPENSE: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_00e3);

fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, 12, 0, 0).unwrap()
}

fn user(id: Uuid, name: &str) -> User {
    User {
        id,
        name: name.into(),
        full_name: name.into(),
        password_hash: Some("x".into()),
    }
}

fn account(id: Uuid, user_id: Uuid, name: &str, ownership_fraction: f64) -> Account {
    Account {
        id,
        user_id,
        name: name.into(),
        description: String::new(),
        color: None,
        iban: None,
        kind: AccountKind::Debit,
        availability: AccountAvailability::Immediately,
        risk: AccountRisk::None,
        opened: at(2025, 1, 1),
        closed: None,
        ownership_fraction,
        sync_account_id: None,
        sync_invert: None,
        renditier_account_id: None,
    }
}

fn expense(id: Uuid, title: &str, transactions: Vec<ExpenseTransaction>) -> Expense {
    Expense {
        id,
        title: title.into(),
        store: String::new(),
        description: String::new(),
        comments: String::new(),
        reference_amount: None,
        accrual_start: at(2026, 6, 1),
        accrual_end: at(2026, 6, 1),
        is_template: false,
        is_tax_relevant: false,
        is_unchecked: false,
        is_incomplete: false,
        is_extraordinary: false,
        receipts: Vec::new(),
        categories: Vec::new(),
        transactions,
        positions: Vec::new(),
        created_at: at(2026, 6, 1),
        updated_at: at(2026, 6, 1),
    }
}

fn leg(account_id: Uuid, amount: i32) -> ExpenseTransaction {
    ExpenseTransaction {
        account_id,
        date: at(2026, 6, 1),
        booking_date: None,
        amount: Some(amount),
        fraction: None,
        comments: String::new(),
        statement: String::new(),
    }
}

fn balance(id: Uuid, account_id: Uuid, amount: i32) -> Balance {
    Balance {
        id,
        account_id,
        date: at(2026, 7, 1),
        amount,
        comments: String::new(),
        created_at: at(2026, 7, 1),
        updated_at: at(2026, 7, 1),
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

    diesel::insert_into(users::table)
        .values(vec![user(MINE, "mine"), user(THEIRS, "theirs")])
        .execute(conn)
        .await
        .expect("users");

    diesel::insert_into(accounts::table)
        .values(vec![
            account(MY_ACCOUNT, MINE, "meins", 1.0),
            account(THEIR_ACCOUNT, THEIRS, "ihres", 1.0),
            // the mirrored pair: half-owned on my side, so a rendering that reached for the
            // wrong account's ownership fraction would still look plausible
            account(MY_MIRROR, MINE, "Schulden", 0.5),
            account(THEIR_MIRROR, THEIRS, "Schulden", 1.0),
        ])
        .execute(conn)
        .await
        .expect("accounts");

    // linked after the fact, because the reference is mutual
    for (from, to) in [(MY_MIRROR, THEIR_MIRROR), (THEIR_MIRROR, MY_MIRROR)] {
        diesel::update(accounts::table.find(from))
            .set((
                accounts::sync_account_id.eq(to),
                accounts::sync_invert.eq(true),
            ))
            .execute(conn)
            .await
            .expect("synchronization");
    }

    for e in [
        expense(MY_EXPENSE, "meine Ausgabe", vec![leg(MY_ACCOUNT, -1000)]),
        expense(
            THEIR_EXPENSE,
            "ihre Ausgabe",
            vec![leg(THEIR_ACCOUNT, -2000)],
        ),
        // one expense touching both users' accounts, which is what renders twice
        expense(
            SHARED_EXPENSE,
            "geteilte Ausgabe",
            vec![leg(MY_ACCOUNT, -600), leg(THEIR_ACCOUNT, -400)],
        ),
    ] {
        moneta::queries::insert_expense(conn, &e)
            .await
            .expect("expense");
    }

    diesel::insert_into(balances::table)
        .values(vec![
            balance(Uuid::from_u128(0xb1), MY_ACCOUNT, 5000),
            balance(Uuid::from_u128(0xb2), THEIR_ACCOUNT, 9000),
        ])
        .execute(conn)
        .await
        .expect("balances");
}

/// The suite is one test rather than several: they share one database, and a second test
/// truncating it while the first is reading would be a race with a very confusing failure.
#[tokio::test]
async fn scoped_accessors_keep_two_users_apart() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("TEST_DATABASE_URL is not set -- skipping the database-backed scoping tests");
        return;
    };

    moneta::migrate(&url).await.expect("migrations");
    let pool = moneta::pool(&url, "moneta-scoping-test").expect("pool");
    let conn = &mut pool.get().await.expect("connection");

    fixtures(conn).await;

    // --- expenses -------------------------------------------------------------------------

    let mine: Vec<Uuid> = scoped::expenses(MINE)
        .select(expenses_rendered::id)
        .load(conn)
        .await
        .expect("my expenses");

    assert!(mine.contains(&MY_EXPENSE), "my own expense is missing");
    assert!(
        mine.contains(&SHARED_EXPENSE),
        "an expense touching my account is mine to see"
    );
    assert!(
        !mine.contains(&THEIR_EXPENSE),
        "an expense that touches none of my accounts leaked into my rendering"
    );

    // an expense across two users renders once per user, and each reader gets exactly one row
    assert_eq!(
        mine.iter().filter(|id| **id == SHARED_EXPENSE).count(),
        1,
        "the shared expense rendered more than once for one reader"
    );

    // --- what a shared expense costs each of them -----------------------------------------

    async fn effective(conn: &mut AsyncPgConnection, user_id: Uuid) -> f64 {
        scoped::expenses(user_id)
            .filter(expenses_rendered::id.eq(SHARED_EXPENSE))
            .select(RenderedExpense::as_select())
            .first(conn)
            .await
            .expect("the shared expense")
            .effective_amount
    }

    // only the leg on their own account counts towards each of them
    assert_eq!(effective(conn, MINE).await, -600.0);
    assert_eq!(effective(conn, THEIRS).await, -400.0);

    // --- transactions ---------------------------------------------------------------------

    // reading these without the filter is what double-counts: each user's rendering of a shared
    // expense lists *all* of its legs, and filtering on the account does not save you
    let my_legs: i64 = scoped::expense_transactions(MINE)
        .filter(expense_transactions_rendered::expense_id.eq(SHARED_EXPENSE))
        .count()
        .get_result(conn)
        .await
        .expect("my legs");

    assert_eq!(my_legs, 2, "my rendering lists both legs");
    let unscoped: i64 = expense_transactions_rendered::table
        .filter(expense_transactions_rendered::expense_id.eq(SHARED_EXPENSE))
        .count()
        .get_result(conn)
        .await
        .expect("every rendering of the legs");
    assert_eq!(
        unscoped, 4,
        "two legs times two readers -- this is the double count the filter prevents"
    );

    // --- balances -------------------------------------------------------------------------

    let my_balances: Vec<RenderedBalance> = scoped::balances(MINE)
        .select(RenderedBalance::as_select())
        .load(conn)
        .await
        .expect("my balances");

    assert!(
        my_balances.iter().all(|b| b.account_id != THEIR_ACCOUNT),
        "a balance on an account of theirs showed up in mine"
    );

    // the mirrored account is the exception, and it is deliberate: their balance on it is my
    // balance too, with the sign flipped
    diesel::insert_into(balances::table)
        .values(balance(Uuid::from_u128(0xb3), THEIR_MIRROR, 5000))
        .execute(conn)
        .await
        .expect("a balance on the mirrored account");

    let mirrored: Vec<RenderedBalance> = scoped::balances(MINE)
        .filter(balances_rendered::account_id.eq(MY_MIRROR))
        .select(RenderedBalance::as_select())
        .load(conn)
        .await
        .expect("my side of the mirror");

    assert_eq!(mirrored.len(), 1, "the mirrored balance is mine to see");
    assert_eq!(
        mirrored[0].amount, -5000,
        "an inverted mirror flips the sign: what they are owed, I owe"
    );

    // --- accounts and the write-side accessors --------------------------------------------

    let my_accounts: Vec<Uuid> = scoped::accounts(MINE)
        .select(accounts::id)
        .load(conn)
        .await
        .expect("my accounts");

    assert!(my_accounts.contains(&MY_ACCOUNT));
    assert!(
        !my_accounts.contains(&THEIR_ACCOUNT),
        "ownership is not the same as visibility, and this one is ownership"
    );

    // the accessor a mutation writes through: their row is not addressable as mine
    let stolen: Option<Uuid> = scoped::own_account(MINE, THEIR_ACCOUNT)
        .select(accounts::id)
        .first(conn)
        .await
        .optional()
        .expect("query");
    assert_eq!(
        stolen, None,
        "own_account handed over an account belonging to somebody else"
    );

    let owned: Option<Uuid> = scoped::own_account(MINE, MY_ACCOUNT)
        .select(accounts::id)
        .first(conn)
        .await
        .optional()
        .expect("query");
    assert_eq!(owned, Some(MY_ACCOUNT));

    // a balance on their account is not mine to write to, even though the id is guessable
    let their_balance: Option<Uuid> = scoped::owned_balances(MINE)
        .filter(balances::id.eq(Uuid::from_u128(0xb2)))
        .select(balances::id)
        .first(conn)
        .await
        .optional()
        .expect("query");
    assert_eq!(
        their_balance, None,
        "owned_balances reached a balance on somebody else's account"
    );
}

/// The test above returns early without a database, which is the right thing locally and the
/// wrong thing in the job that exists to run it: a suite that silently tests nothing is worse
/// than one that is missing, because it reports green.
///
/// Keyed on a variable of its own rather than on `CI`, which every job sets — including the ones
/// that have no database on purpose and whose skip is the correct outcome.
#[test]
fn the_database_tests_are_not_quietly_skipped() {
    if std::env::var("MONETA_REQUIRE_DB_TESTS").is_err() {
        return;
    }

    assert!(
        std::env::var("TEST_DATABASE_URL").is_ok(),
        "this job exists to run the scoping tests, but TEST_DATABASE_URL is unset and they \
         skipped themselves"
    );
}
