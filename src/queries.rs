use crate::models::*;
use crate::schema::*;

use diesel::prelude::*;

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
    offset: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<(Balance, Option<AccountSynchronization>)>, diesel::result::Error> {
    balances::table
        .left_join(
            account_synchronizations::table.on(balances::account_id
                .eq(account_synchronizations::account1)
                .or(balances::account_id.eq(account_synchronizations::account2))),
        )
        .left_join(accounts::table.on(accounts::id.eq(balances::account_id)))
        .filter(
            accounts::user_id
                .eq(user_id)
                .or(account_synchronizations::user1.eq(user_id))
                .or(account_synchronizations::user2.eq(user_id)),
        )
        .distinct()
        .select((
            balances::all_columns,
            account_synchronizations::all_columns.nullable(),
        ))
        .limit(limit.unwrap_or(i64::MAX))
        .offset(offset.unwrap_or(0))
        .load::<(Balance, Option<AccountSynchronization>)>(connection)
}

pub fn relevant_balance_by_id(
    connection: &PgConnection,
    user_id: i32,
    id: i32,
) -> Result<Option<(Balance, Option<AccountSynchronization>)>, diesel::result::Error> {
    balances::table
        .left_join(
            account_synchronizations::table.on((account_synchronizations::account1
                .eq(balances::account_id)
                .and(account_synchronizations::user1.ne(user_id)))
            .or(account_synchronizations::account2
                .eq(balances::account_id)
                .and(account_synchronizations::user2.ne(user_id)))),
        )
        .left_join(accounts::table.on(accounts::id.eq(balances::account_id)))
        .filter(
            accounts::user_id
                .eq(user_id)
                .or(account_synchronizations::user1.eq(user_id))
                .or(account_synchronizations::user2.eq(user_id)),
        )
        .filter(balances::id.eq(id))
        .distinct()
        .select((
            balances::all_columns,
            account_synchronizations::all_columns.nullable(),
        ))
        .get_result::<(Balance, Option<AccountSynchronization>)>(connection)
        .optional()
}

pub fn relevant_expenses(
    connection: &PgConnection,
    user_id: i32,
    offset: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<Expense>, diesel::result::Error> {
    expenses::table
        .select(expenses::all_columns)
        .distinct()
        .left_join(
            expense_transactions::table.on(expense_transactions::expense_id.eq(expenses::id)),
        )
        .left_join(
            account_synchronizations::table.on(expense_transactions::account_id
                .eq(account_synchronizations::account1)
                .or(expense_transactions::account_id.eq(account_synchronizations::account2))),
        )
        .left_join(accounts::table.on(accounts::id.eq(expense_transactions::account_id)))
        .filter(
            accounts::user_id
                .eq(user_id)
                .or(account_synchronizations::user1.eq(user_id))
                .or(account_synchronizations::user2.eq(user_id)),
        )
        .limit(limit.unwrap_or(i64::MAX))
        .offset(offset.unwrap_or(0))
        .load::<Expense>(connection)
}

pub fn relevant_expense_by_id(
    connection: &PgConnection,
    user_id: i32,
    id: i32,
) -> Result<Option<Expense>, diesel::result::Error> {
    expenses::table
        .select(expenses::all_columns)
        .distinct()
        .left_join(
            expense_transactions::table.on(expense_transactions::expense_id.eq(expenses::id)),
        )
        .left_join(
            account_synchronizations::table.on(expense_transactions::account_id
                .eq(account_synchronizations::account1)
                .or(expense_transactions::account_id.eq(account_synchronizations::account2))),
        )
        .left_join(accounts::table.on(accounts::id.eq(expense_transactions::account_id)))
        .filter(
            accounts::user_id
                .eq(user_id)
                .or(account_synchronizations::user1.eq(user_id))
                .or(account_synchronizations::user2.eq(user_id)),
        )
        .filter(expenses::id.eq(id))
        .get_result::<Expense>(connection)
        .optional()
}

pub fn expense_transactions_by_expense_id(
    connection: &PgConnection,
    user_id: i32,
    expense_id: i32,
) -> Result<Vec<(ExpenseTransaction, Account, Option<AccountSynchronization>)>, diesel::result::Error>
{
    expense_transactions::table
        .inner_join(accounts::table.on(accounts::id.eq(expense_transactions::account_id)))
        .left_join(
            account_synchronizations::table.on((account_synchronizations::account1
                .eq(expense_transactions::account_id)
                .and(account_synchronizations::user1.ne(user_id)))
            .or(account_synchronizations::account2
                .eq(expense_transactions::account_id)
                .and(account_synchronizations::user2.ne(user_id)))),
        )
        .filter(expense_transactions::expense_id.eq(expense_id))
        .load::<(ExpenseTransaction, Account, Option<AccountSynchronization>)>(connection)
}

pub fn expense_categories_by_expense_id(
    connection: &PgConnection,
    user_id: i32,
    expense_id: i32,
) -> Result<Vec<(ExpenseCategory, Option<CategoryReplacement>)>, diesel::result::Error> {
    expense_categories::table
        .inner_join(categories::table.on(categories::id.eq(expense_categories::category_id)))
        .left_join(
            category_replacements::table.on(category_replacements::original
                .eq(categories::id)
                .and(category_replacements::user_id.eq(user_id))),
        )
        .filter(expense_categories::expense_id.eq(expense_id))
        .select((
            expense_categories::all_columns,
            category_replacements::all_columns.nullable(),
        ))
        .load::<(ExpenseCategory, Option<CategoryReplacement>)>(connection)
}

// needed data for expenses returned by `relevant_expenses`
pub fn expense_categories_by_expense_range(
    connection: &PgConnection,
    user_id: i32,
    offset: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<(ExpenseCategory, Option<CategoryReplacement>)>, diesel::result::Error> {
    expenses::table
        .left_join(
            expense_transactions::table.on(expense_transactions::expense_id.eq(expenses::id)),
        )
        .left_join(
            account_synchronizations::table.on(expense_transactions::account_id
                .eq(account_synchronizations::account1)
                .or(expense_transactions::account_id.eq(account_synchronizations::account2))),
        )
        .left_join(accounts::table.on(accounts::id.eq(expense_transactions::account_id)))
        .filter(
            accounts::user_id
                .eq(user_id)
                .or(account_synchronizations::user1.eq(user_id))
                .or(account_synchronizations::user2.eq(user_id)),
        )
        .limit(limit.unwrap_or(i64::MAX))
        .offset(offset.unwrap_or(0))
        .inner_join(expense_categories::table.on(expense_categories::expense_id.eq(expenses::id)))
        .inner_join(categories::table.on(categories::id.eq(expense_categories::category_id)))
        .left_join(
            category_replacements::table.on(category_replacements::original
                .eq(categories::id)
                .and(category_replacements::user_id.eq(user_id))),
        )
        .select((
            expense_categories::all_columns,
            category_replacements::all_columns.nullable(),
        ))
        .distinct()
        .load::<(ExpenseCategory, Option<CategoryReplacement>)>(connection)
}

// needed data for expenses returned by `relevant_expenses`
pub fn expense_transactions_by_expense_range(
    connection: &PgConnection,
    user_id: i32,
    offset: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<(ExpenseTransaction, Account, Option<AccountSynchronization>)>, diesel::result::Error>
{
    // TODO: this works for small counts, but might not work when requesting all expense transactions!

    let expenses = relevant_expenses(connection, user_id, offset, limit)?;

    expense_transactions::table
        .inner_join(accounts::table.on(accounts::id.eq(expense_transactions::account_id)))
        .left_join(
            account_synchronizations::table.on((account_synchronizations::account1
                .eq(expense_transactions::account_id)
                .and(account_synchronizations::user1.ne(user_id)))
            .or(account_synchronizations::account2
                .eq(expense_transactions::account_id)
                .and(account_synchronizations::user2.ne(user_id)))),
        )
        .order(expense_transactions::expense_id.asc())
        .filter(
            expense_transactions::expense_id
                .eq_any(expenses.into_iter().map(|e| e.id).collect::<Vec<_>>()),
        )
        .load::<(ExpenseTransaction, Account, Option<AccountSynchronization>)>(connection)
}

// needed data for expenses returned by `relevant_expenses`
pub fn expense_events_by_expense_range(
    connection: &PgConnection,
    user_id: i32,
    offset: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<ExpenseEvent>, diesel::result::Error> {
    expenses::table
        .left_join(
            expense_transactions::table.on(expense_transactions::expense_id.eq(expenses::id)),
        )
        .left_join(
            account_synchronizations::table.on(expense_transactions::account_id
                .eq(account_synchronizations::account1)
                .or(expense_transactions::account_id.eq(account_synchronizations::account2))),
        )
        .left_join(accounts::table.on(accounts::id.eq(expense_transactions::account_id)))
        .filter(
            accounts::user_id
                .eq(user_id)
                .or(account_synchronizations::user1.eq(user_id))
                .or(account_synchronizations::user2.eq(user_id)),
        )
        .limit(limit.unwrap_or(i64::MAX))
        .offset(offset.unwrap_or(0))
        .inner_join(expense_events::table.on(expense_events::expense_id.eq(expenses::id)))
        .select(expense_events::all_columns)
        .distinct()
        .load::<ExpenseEvent>(connection)
}

// needed data for expenses returned by `relevant_expenses`
pub fn expense_receipts_by_expense_range(
    connection: &PgConnection,
    user_id: i32,
    offset: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<ExpenseReceipt>, diesel::result::Error> {
    expenses::table
        .left_join(
            expense_transactions::table.on(expense_transactions::expense_id.eq(expenses::id)),
        )
        .left_join(
            account_synchronizations::table.on(expense_transactions::account_id
                .eq(account_synchronizations::account1)
                .or(expense_transactions::account_id.eq(account_synchronizations::account2))),
        )
        .left_join(accounts::table.on(accounts::id.eq(expense_transactions::account_id)))
        .filter(
            accounts::user_id
                .eq(user_id)
                .or(account_synchronizations::user1.eq(user_id))
                .or(account_synchronizations::user2.eq(user_id)),
        )
        .limit(limit.unwrap_or(i64::MAX))
        .offset(offset.unwrap_or(0))
        .inner_join(expense_receipts::table.on(expense_receipts::expense_id.eq(expenses::id)))
        .select(expense_receipts::all_columns)
        .distinct()
        .load::<ExpenseReceipt>(connection)
}

pub fn relevant_transactions(
    connection: &PgConnection,
    user_id: i32,
) -> Result<Vec<ExpenseTransaction>, diesel::result::Error> {
    expenses::table
        .left_join(
            expense_transactions::table.on(expense_transactions::expense_id.eq(expenses::id)),
        )
        .left_join(
            account_synchronizations::table.on(expense_transactions::account_id
                .eq(account_synchronizations::account1)
                .or(expense_transactions::account_id.eq(account_synchronizations::account2))),
        )
        .left_join(accounts::table.on(accounts::id.eq(expense_transactions::account_id)))
        .filter(
            accounts::user_id
                .eq(user_id)
                .or(account_synchronizations::user1.eq(user_id))
                .or(account_synchronizations::user2.eq(user_id)),
        )
        .select(expense_transactions::all_columns)
        .distinct()
        .load::<ExpenseTransaction>(connection)
}
