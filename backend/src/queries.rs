use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::models::{Account, Category, User};
use crate::schema::{accounts, categories, users};

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
