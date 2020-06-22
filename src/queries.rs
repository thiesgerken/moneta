use crate::models::*;
use crate::schema::*;

use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::Integer;

pub fn relevant_accounts(
    connection: &PgConnection,
    user_id: i32,
) -> Result<Vec<(Account, Option<AccountSynchronization>)>, diesel::result::Error> {
    sql_query(include_str!("sql/relevant_accounts.sql"))
        .bind::<Integer, _>(user_id)
        .load::<(Account, Option<AccountSynchronization>)>(connection)
}

pub fn accounts(
    connection: &PgConnection,
    offset: Option<i32>,
    limit: Option<i32>,
) -> Result<Vec<(Account, Option<AccountSynchronization>)>, diesel::result::Error> {
    accounts::table
        .left_join(
            account_synchronizations::table.on(accounts::id
                .eq(account_synchronizations::account1)
                .or(accounts::id.eq(account_synchronizations::account2))),
        )
        .order(accounts::id)
        .limit(limit.unwrap_or(i32::MAX) as i64)
        .offset(offset.unwrap_or(0) as i64)
        .load::<(Account, Option<AccountSynchronization>)>(connection)
}

pub fn account_by_id(
    connection: &PgConnection,
    id: i32,
) -> Result<Option<(Account, Option<AccountSynchronization>)>, diesel::result::Error> {
    accounts::table
        .filter(accounts::id.eq(id))
        .left_join(
            account_synchronizations::table.on(accounts::id
                .eq(account_synchronizations::account1)
                .or(accounts::id.eq(account_synchronizations::account2))),
        )
        .get_result::<(Account, Option<AccountSynchronization>)>(connection)
        .optional()
}

pub fn relevant_balances(
    connection: &PgConnection,
    user_id: i32,
    offset: Option<i32>,
    limit: Option<i32>,
) -> Result<Vec<(Balance, Option<AccountSynchronization>)>, diesel::result::Error> {
    sql_query(include_str!("sql/relevant_balances.sql"))
        .bind::<Integer, _>(user_id)
        .bind::<Integer, _>(limit.unwrap_or(i32::MAX))
        .bind::<Integer, _>(offset.unwrap_or(0))
        .load::<(Balance, Option<AccountSynchronization>)>(connection)
}

pub fn relevant_balance_by_id(
    connection: &PgConnection,
    user_id: i32,
    id: i32,
) -> Result<Option<(Balance, Option<AccountSynchronization>)>, diesel::result::Error> {
    sql_query(include_str!("sql/relevant_balance_by_id.sql"))
        .bind::<Integer, _>(user_id)
        .bind::<Integer, _>(id)
        .get_result::<(Balance, Option<AccountSynchronization>)>(connection)
        .optional()
}

pub fn relevant_expenses(
    connection: &PgConnection,
    user_id: i32,
    offset: Option<i32>,
    limit: Option<i32>,
) -> Result<Vec<Expense>, diesel::result::Error> {
    sql_query(include_str!("sql/expense_ranges_simple/expense_range.sql"))
        .bind::<Integer, _>(user_id)
        .bind::<Integer, _>(limit.unwrap_or(i32::MAX))
        .bind::<Integer, _>(offset.unwrap_or(0))
        .load::<Expense>(connection)
}

pub fn relevant_expense_by_id(
    connection: &PgConnection,
    user_id: i32,
    id: i32,
) -> Result<Option<Expense>, diesel::result::Error> {
    sql_query(include_str!("sql/relevant_expense_by_id.sql"))
        .bind::<Integer, _>(user_id)
        .bind::<Integer, _>(id)
        .get_result::<Expense>(connection)
        .optional()
}

pub fn expense_transactions_by_expense_id(
    connection: &PgConnection,
    user_id: i32,
    id: i32,
) -> Result<Vec<(ExpenseTransaction, Option<AccountSynchronization>)>, diesel::result::Error> {
    sql_query(include_str!("sql/expense_transactions_by_expense_id.sql"))
        .bind::<Integer, _>(user_id)
        .bind::<Integer, _>(id)
        .load::<(ExpenseTransaction, Option<AccountSynchronization>)>(connection)
}

pub fn expense_categories_by_expense_id(
    connection: &PgConnection,
    user_id: i32,
    id: i32,
) -> Result<Vec<(ExpenseCategory, Option<CategoryReplacement>)>, diesel::result::Error> {
    sql_query(include_str!("sql/expense_categories_by_expense_id.sql"))
        .bind::<Integer, _>(user_id)
        .bind::<Integer, _>(id)
        .load::<(ExpenseCategory, Option<CategoryReplacement>)>(connection)
}

// needed data for expenses returned by `relevant_expenses`
pub fn expense_categories_by_expense_range(
    connection: &PgConnection,
    user_id: i32,
    offset: Option<i32>,
    limit: Option<i32>,
) -> Result<Vec<(ExpenseCategory, Option<CategoryReplacement>)>, diesel::result::Error> {
    sql_query(include_str!(
        "sql/expense_ranges_simple/expense_categories_by_expense_range.sql"
    ))
    .bind::<Integer, _>(user_id)
    .bind::<Integer, _>(limit.unwrap_or(i32::MAX))
    .bind::<Integer, _>(offset.unwrap_or(0))
    .load::<(ExpenseCategory, Option<CategoryReplacement>)>(connection)
}

// needed data for expenses returned by `relevant_expenses`
pub fn expense_transactions_by_expense_range(
    connection: &PgConnection,
    user_id: i32,
    offset: Option<i32>,
    limit: Option<i32>,
) -> Result<Vec<(ExpenseTransaction, Option<AccountSynchronization>)>, diesel::result::Error> {
    sql_query(include_str!(
        "sql/expense_ranges_simple/expense_transactions_by_expense_range.sql"
    ))
    .bind::<Integer, _>(user_id)
    .bind::<Integer, _>(limit.unwrap_or(i32::MAX))
    .bind::<Integer, _>(offset.unwrap_or(0))
    .load::<(ExpenseTransaction, Option<AccountSynchronization>)>(connection)
}

// needed data for expenses returned by `relevant_expenses`
pub fn expense_events_by_expense_range(
    connection: &PgConnection,
    user_id: i32,
    offset: Option<i32>,
    limit: Option<i32>,
) -> Result<Vec<ExpenseEvent>, diesel::result::Error> {
    sql_query(include_str!(
        "sql/expense_ranges_simple/expense_events_by_expense_range.sql"
    ))
    .bind::<Integer, _>(user_id)
    .bind::<Integer, _>(limit.unwrap_or(i32::MAX))
    .bind::<Integer, _>(offset.unwrap_or(0))
    .load::<ExpenseEvent>(connection)
}

// needed data for expenses returned by `relevant_expenses`
pub fn expense_receipts_by_expense_range(
    connection: &PgConnection,
    user_id: i32,
    offset: Option<i32>,
    limit: Option<i32>,
) -> Result<Vec<ExpenseReceipt>, diesel::result::Error> {
    sql_query(include_str!(
        "sql/expense_ranges_simple/expense_receipts_by_expense_range.sql"
    ))
    .bind::<Integer, _>(user_id)
    .bind::<Integer, _>(limit.unwrap_or(i32::MAX))
    .bind::<Integer, _>(offset.unwrap_or(0))
    .load::<ExpenseReceipt>(connection)
}

pub fn relevant_transactions(
    connection: &PgConnection,
    user_id: i32,
) -> Result<Vec<ExpenseTransaction>, diesel::result::Error> {
    sql_query(include_str!("sql/relevant_transactions.sql"))
        .bind::<Integer, _>(user_id)
        .load::<ExpenseTransaction>(connection)
}
