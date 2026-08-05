use uuid::Uuid;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::models::{
    Account, Category, Expense, ExpenseCategorizationRow, ExpenseRow, ExpenseTransactionRow, User,
};
use crate::schema::{
    accounts, categories, expense_categorizations, expense_transactions, expenses, users,
};

/// Every account a change to `account_ids` could affect: the accounts themselves plus the ones
/// mirroring them, since a mirror shows the same money in another user's books.
pub async fn query_associated_accounts(
    conn: &mut AsyncPgConnection,
    account_ids: &[Uuid],
) -> QueryResult<Vec<Account>> {
    accounts::table
        .filter(
            accounts::id
                .eq_any(account_ids)
                .or(accounts::sync_account_id.eq_any(account_ids)),
        )
        .select(Account::as_select())
        .load(conn)
        .await
}

pub async fn query_user_by_name(
    conn: &mut AsyncPgConnection,
    user_name: &str,
) -> QueryResult<Option<User>> {
    users::table
        .filter(users::name.eq(user_name))
        .select(User::as_select())
        .first(conn)
        .await
        .optional()
}

pub async fn query_categories(conn: &mut AsyncPgConnection) -> QueryResult<Vec<Category>> {
    categories::table
        .select(Category::as_select())
        .load(conn)
        .await
}

/// Reassembles the aggregate the changelog and the importer work with out of its three tables.
pub async fn load_expense(conn: &mut AsyncPgConnection, id: Uuid) -> QueryResult<Option<Expense>> {
    let Some(row) = expenses::table
        .find(id)
        .select(ExpenseRow::as_select())
        .first(conn)
        .await
        .optional()?
    else {
        return Ok(None);
    };

    let transactions = expense_transactions::table
        .filter(expense_transactions::expense_id.eq(id))
        .order_by(expense_transactions::idx.asc())
        .select(ExpenseTransactionRow::as_select())
        .load(conn)
        .await?;

    let categories = expense_categorizations::table
        .filter(expense_categorizations::expense_id.eq(id))
        .order_by(expense_categorizations::category_id.asc())
        .select(ExpenseCategorizationRow::as_select())
        .load(conn)
        .await?;

    Ok(Some(Expense::join(row, transactions, categories)))
}

/// Writes the whole aggregate. Call inside a transaction: three tables have to move together or
/// an expense ends up with somebody else's transactions.
pub async fn insert_expense(conn: &mut AsyncPgConnection, expense: &Expense) -> QueryResult<()> {
    let (row, transactions, categories) = expense.split();

    diesel::insert_into(expenses::table)
        .values(&row)
        .execute(conn)
        .await?;
    diesel::insert_into(expense_transactions::table)
        .values(&transactions)
        .execute(conn)
        .await?;
    if !categories.is_empty() {
        diesel::insert_into(expense_categorizations::table)
            .values(&categories)
            .execute(conn)
            .await?;
    }

    Ok(())
}

/// Replaces an expense in place, keeping its id. The children are deleted and rewritten rather
/// than diffed: `idx` has to stay dense and gapless, and a diff that got that wrong would
/// reorder the transaction list instead of failing.
pub async fn replace_expense(conn: &mut AsyncPgConnection, expense: &Expense) -> QueryResult<()> {
    let (row, transactions, categories) = expense.split();

    diesel::update(expenses::table.find(expense.id))
        .set(&row)
        .execute(conn)
        .await?;

    diesel::delete(expense_transactions::table.filter(expense_transactions::expense_id.eq(expense.id)))
        .execute(conn)
        .await?;
    diesel::delete(
        expense_categorizations::table.filter(expense_categorizations::expense_id.eq(expense.id)),
    )
    .execute(conn)
    .await?;

    diesel::insert_into(expense_transactions::table)
        .values(&transactions)
        .execute(conn)
        .await?;
    if !categories.is_empty() {
        diesel::insert_into(expense_categorizations::table)
            .values(&categories)
            .execute(conn)
            .await?;
    }

    Ok(())
}
