use std::collections::HashMap;

use uuid::Uuid;

use crate::graphql::root::{Context, require_user};
use crate::graphql::util::{self, SortDirection};
use crate::models::AccountAvailability;
use crate::models::AccountKind;
use crate::models::AccountRisk;
use crate::models::AccountSynchronization;
use crate::models::{Account, AccountCreateInput, AccountUpdateInput};
use crate::order_by;
use crate::schema::{accounts, balances, expense_transactions};

use chrono::DateTime;
use chrono::Utc;
use dataloader::BatchFn;
use dataloader::cached::Loader;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use juniper::{FieldResult, GraphQLEnum, graphql_object};

use super::users::GraphQLUserInfo;

#[derive(Debug, Clone)]
pub struct GraphQLAccountSynchronization {
    pub data: AccountSynchronization,
}

#[graphql_object(Context = Context, name = "AccountSynchronization")]
impl GraphQLAccountSynchronization {
    fn invert(&self) -> bool {
        self.data.invert
    }

    async fn account(&self, context: &Context) -> FieldResult<GraphQLAccount> {
        Ok(GraphQLAccount::new(
            context.account_loader.try_load(self.data.account_id).await?,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct GraphQLAccount {
    pub data: Account,
}

impl GraphQLAccount {
    pub fn new(data: Account) -> Self {
        Self { data }
    }
}

#[graphql_object(Context = Context, name = "Account")]
impl GraphQLAccount {
    fn id(&self) -> Uuid {
        self.data.id
    }

    async fn owner(&self, context: &Context) -> FieldResult<GraphQLUserInfo> {
        Ok(GraphQLUserInfo::new(
            context.user_loader.try_load(self.data.user_id).await?,
        ))
    }

    fn name(&self) -> &str {
        &self.data.name
    }

    fn description(&self) -> &str {
        &self.data.description
    }

    fn color(&self) -> &Option<String> {
        &self.data.color
    }

    fn iban(&self) -> &Option<String> {
        &self.data.iban
    }

    fn kind(&self) -> AccountKind {
        self.data.kind
    }

    fn availability(&self) -> AccountAvailability {
        self.data.availability
    }

    fn risk(&self) -> AccountRisk {
        self.data.risk
    }

    fn opened(&self) -> DateTime<Utc> {
        self.data.opened
    }

    fn closed(&self) -> Option<DateTime<Utc>> {
        self.data.closed
    }

    fn ownership_fraction(&self) -> f64 {
        self.data.ownership_fraction
    }

    fn synchronization(&self) -> Option<GraphQLAccountSynchronization> {
        self.data
            .synchronization()
            .map(|data| GraphQLAccountSynchronization { data })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, GraphQLEnum)]
pub enum AccountSortField {
    #[default]
    Name,
    Kind,
    Availability,
    Risk,
    Opened,
    Closed,
}

pub struct AccountQuery;

#[graphql_object(
    Context = Context,
)]
impl AccountQuery {
    pub async fn list(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
        search: Option<String>,
        only_open: Option<bool>,
        sort_by: Option<AccountSortField>,
        sort_dir: Option<SortDirection>,
    ) -> FieldResult<Vec<GraphQLAccount>> {
        require_user(context).await?;
        let conn = &mut context.conn().await?;

        let (limit, offset) = util::page(limit, skip);
        let direction = sort_dir.unwrap_or_default();

        let mut query = accounts::table.into_boxed();
        if let Some(term) = search.filter(|s| !s.trim().is_empty()) {
            let pattern = util::ilike_pattern(term.trim());
            query = query.filter(
                accounts::name
                    .ilike(pattern.clone())
                    .or(accounts::description.ilike(pattern.clone()))
                    .or(accounts::iban.ilike(pattern)),
            );
        }
        if only_open.unwrap_or(false) {
            query = query.filter(accounts::closed.is_null().or(accounts::closed.gt(Utc::now())));
        }

        let query = match sort_by.unwrap_or_default() {
            AccountSortField::Name => order_by!(query, direction, accounts::name),
            AccountSortField::Kind => order_by!(query, direction, accounts::kind),
            AccountSortField::Availability => order_by!(query, direction, accounts::availability),
            AccountSortField::Risk => order_by!(query, direction, accounts::risk),
            AccountSortField::Opened => order_by!(query, direction, accounts::opened),
            AccountSortField::Closed => order_by!(query, direction, accounts::closed),
        };

        let found = query
            .limit(limit)
            .offset(offset)
            .select(Account::as_select())
            .load(conn)
            .await?;

        context
            .account_loader
            .prime_many(found.iter().map(|a| (a.id, a.clone())))
            .await;

        Ok(found.into_iter().map(GraphQLAccount::new).collect())
    }

    pub async fn get(context: &Context, id: Uuid) -> FieldResult<GraphQLAccount> {
        require_user(context).await?;
        Ok(GraphQLAccount::new(
            context.account_loader.try_load(id).await?,
        ))
    }
}

pub struct AccountMutation;

#[graphql_object(
    Context = Context,
)]
impl AccountMutation {
    pub async fn delete(context: &Context, id: Uuid) -> FieldResult<bool> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let account = accounts::table
            .find(id)
            .filter(accounts::user_id.eq(user_id))
            .select(Account::as_select())
            .first(conn)
            .await
            .optional()?
            .ok_or("entry to delete not found")?;

        if account.synchronization().is_some() {
            return Err(format!(
                "Deletion of synchronized accounts is unsupported, tried to remove {id}"
            )
            .into());
        }

        // Balances are ON DELETE CASCADE, so nothing stops this at the database level -- they
        // would just disappear along with the account. Refuse instead, since losing them is not
        // what someone deleting an account is asking for.
        let balance_count: i64 = balances::table
            .filter(balances::account_id.eq(id))
            .count()
            .get_result(conn)
            .await?;

        if balance_count > 0 {
            return Err(format!(
                "Tried to remove account {id}, but it still has {balance_count} balances attached to it"
            ).into());
        }

        // The foreign key is ON DELETE RESTRICT; this only turns the constraint name into a
        // number the user can act on.
        let transaction_count: i64 = expense_transactions::table
            .filter(expense_transactions::account_id.eq(id))
            .count()
            .get_result(conn)
            .await?;

        if transaction_count > 0 {
            return Err(format!(
                "Tried to remove account {id}, but it still has {transaction_count} expense transactions attached to it"
            ).into());
        }

        let count = diesel::delete(accounts::table.find(id).filter(accounts::user_id.eq(user_id)))
            .execute(conn)
            .await?;
        Ok(count == 1)
    }

    pub async fn create(
        context: &Context,
        account: AccountCreateInput,
    ) -> FieldResult<GraphQLAccount> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        account.validate()?;

        let account = account.upgrade(Uuid::now_v7(), user_id);
        diesel::insert_into(accounts::table)
            .values(&account)
            .execute(conn)
            .await?;

        Ok(GraphQLAccount::new(account))
    }

    pub async fn update(
        context: &Context,
        id: Uuid,
        account: AccountUpdateInput,
    ) -> FieldResult<bool> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        account.validate()?;

        // The synchronization pair is not part of the input, so it is left alone here.
        let count = diesel::update(accounts::table.find(id).filter(accounts::user_id.eq(user_id)))
            .set((
                accounts::name.eq(account.name),
                accounts::description.eq(account.description),
                accounts::color.eq(account.color),
                accounts::iban.eq(account.iban),
                accounts::kind.eq(account.kind),
                accounts::availability.eq(account.availability),
                accounts::risk.eq(account.risk),
                accounts::opened.eq(account.opened),
                accounts::closed.eq(account.closed),
                accounts::ownership_fraction.eq(account.ownership_fraction),
            ))
            .execute(conn)
            .await?;

        Ok(count == 1)
    }
}

pub struct AccountBatcher {
    pool: crate::Pool,
}

impl BatchFn<Uuid, Account> for AccountBatcher {
    async fn load(&mut self, keys: &[Uuid]) -> HashMap<Uuid, Account> {
        debug!("Performing a coalesced account query for {} ids", keys.len());

        let Ok(mut conn) = self.pool.get().await else {
            return HashMap::new();
        };

        accounts::table
            .filter(accounts::id.eq_any(keys))
            .select(Account::as_select())
            .load(&mut conn)
            .await
            .map(|found| found.into_iter().map(|a| (a.id, a)).collect())
            .unwrap_or_default()
    }
}

pub type AccountLoader = Loader<Uuid, Account, AccountBatcher>;

pub fn get_account_loader(pool: crate::Pool) -> AccountLoader {
    Loader::new(AccountBatcher { pool })
}
