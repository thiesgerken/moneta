use crate::models::*;

use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::Integer;

pub fn relevant_accounts(
    connection: &PgConnection,
    user_id: i32,
    offset: Option<i32>,
    limit: Option<i32>,
) -> Result<Vec<(Account, Option<AccountSynchronization>)>, diesel::result::Error> {
    sql_query(include_str!("sql/relevant_accounts.sql"))
        .bind::<Integer, _>(user_id)
        .bind::<Integer, _>(limit.unwrap_or(i32::MAX))
        .bind::<Integer, _>(offset.unwrap_or(0))
        .get_results::<(Account, Option<AccountSynchronization>)>(connection)
}

pub fn relevant_account_by_id(
    connection: &PgConnection,
    user_id: i32,
    id: i32,
) -> Result<Option<(Account, Option<AccountSynchronization>)>, diesel::result::Error> {
    sql_query(include_str!("sql/relevant_account_by_id.sql"))
        .bind::<Integer, _>(user_id)
        .bind::<Integer, _>(id)
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
        .get_results::<(Balance, Option<AccountSynchronization>)>(connection)
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

// TODO: allow filtering for booking date
pub fn relevant_expenses(
    connection: &PgConnection,
    user_id: i32,
) -> Result<Vec<Expense>, diesel::result::Error> {
    sql_query(include_str!("sql/relevant_expenses.sql"))
        .bind::<Integer, _>(user_id)
        .get_results::<Expense>(connection)
}

// TODO: allow filtering for value date
pub fn relevant_transactions(
    connection: &PgConnection,
    user_id: i32,
) -> Result<Vec<ExpenseTransaction>, diesel::result::Error> {
    sql_query(include_str!("sql/relevant_transactions.sql"))
        .bind::<Integer, _>(user_id)
        .get_results::<ExpenseTransaction>(connection)
}
