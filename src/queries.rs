use crate::models::*;

use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::Integer;

pub fn relevant_accounts(
    connection: &PgConnection,
    user_id: i32,
) -> Result<Vec<Account>, diesel::result::Error> {
    sql_query(include_str!("sql/relevant_accounts.sql"))
        .bind::<Integer, _>(user_id)
        .get_results::<Account>(connection)
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
