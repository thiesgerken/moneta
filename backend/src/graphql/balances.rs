use uuid::Uuid;

use crate::graphql::accounts::GraphQLAccount;
use crate::graphql::root::{Context, require_user};
use crate::graphql::util::{self, SortDirection};
use crate::models::Balance;
use crate::models::rendering::{
    RenderedBalance, RenderedBalanceCreateInput, RenderedBalanceUpdateInput,
};
use crate::order_by;
use crate::schema::{accounts, balances};
use crate::schema_views::balances_rendered;

use chrono::DateTime;
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use juniper::{FieldResult, GraphQLEnum, GraphQLObject, graphql_object};

#[derive(Debug, Clone)]
pub struct GraphQLBalance {
    data: RenderedBalance,
}

impl GraphQLBalance {
    pub fn new(data: RenderedBalance) -> Self {
        Self { data }
    }
}

#[graphql_object(Context = Context, name = "Balance")]
impl GraphQLBalance {
    fn id(&self) -> Uuid {
        self.data.id
    }

    fn upstream_id(&self) -> Uuid {
        self.data.upstream_id
    }

    fn comments(&self) -> &str {
        &self.data.comments
    }

    fn amount(&self) -> i32 {
        self.data.amount
    }

    fn date(&self) -> DateTime<Utc> {
        self.data.date
    }

    async fn account(&self, context: &Context) -> FieldResult<GraphQLAccount> {
        Ok(GraphQLAccount::new(
            context.account_loader.try_load(self.data.account_id).await?,
        ))
    }
}

#[derive(GraphQLObject)]
#[graphql(context=Context)]
pub struct BalanceConnection {
    pub total_count: i32,
    pub edges: Vec<BalanceNode>,
}

#[derive(GraphQLObject)]
#[graphql(context=Context)]
pub struct BalanceNode {
    pub node: GraphQLBalance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, GraphQLEnum)]
pub enum BalanceSortField {
    #[default]
    Date,
    Amount,
}

pub struct BalanceQuery;

#[graphql_object(
    Context = Context,
)]
impl BalanceQuery {
    pub async fn list(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
        search: Option<String>,
        account_id: Option<Uuid>,
        sort_by: Option<BalanceSortField>,
        sort_dir: Option<SortDirection>,
    ) -> FieldResult<BalanceConnection> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let (limit, offset) = util::page(limit, skip);
        let direction = sort_dir.unwrap_or_default();
        let pattern = search
            .filter(|s| !s.trim().is_empty())
            .map(|s| util::ilike_pattern(s.trim()));

        // A boxed query is consumed by the count, so the filter is built twice rather than
        // cloned.
        let filtered = || {
            let mut query = balances_rendered::table
                .into_boxed()
                .filter(balances_rendered::user_id.eq(user_id));

            if let Some(pattern) = &pattern {
                query = query.filter(balances_rendered::comments.ilike(pattern.clone()));
            }
            if let Some(account_id) = account_id {
                query = query.filter(balances_rendered::account_id.eq(account_id));
            }

            query
        };

        let total_count: i64 = filtered().count().get_result(conn).await?;

        let query = match sort_by.unwrap_or_default() {
            BalanceSortField::Date => order_by!(filtered(), direction, balances_rendered::date),
            BalanceSortField::Amount => order_by!(filtered(), direction, balances_rendered::amount),
        };

        let found = query
            .limit(limit)
            .offset(offset)
            .select(RenderedBalance::as_select())
            .load(conn)
            .await?;

        Ok(BalanceConnection {
            total_count: total_count as i32,
            edges: found
                .into_iter()
                .map(|node| BalanceNode {
                    node: GraphQLBalance::new(node),
                })
                .collect(),
        })
    }

    pub async fn get(context: &Context, id: Uuid) -> FieldResult<GraphQLBalance> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        Ok(GraphQLBalance::new(load_rendered(conn, user_id, id).await?))
    }
}

pub struct BalanceMutation;

#[graphql_object(
    Context = Context,
)]
impl BalanceMutation {
    pub async fn delete(context: &Context, id: Uuid) -> FieldResult<bool> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        require_own_balance(conn, user_id, id).await?;

        let count = diesel::delete(balances::table.find(id)).execute(conn).await?;
        Ok(count == 1)
    }

    pub async fn create(
        context: &Context,
        balance: RenderedBalanceCreateInput,
    ) -> FieldResult<GraphQLBalance> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        require_own_account(conn, user_id, balance.account_id).await?;

        let entry = Balance::from_rendered(balance);
        let id = entry.id;
        diesel::insert_into(balances::table)
            .values(&entry)
            .execute(conn)
            .await?;

        Ok(GraphQLBalance::new(load_rendered(conn, user_id, id).await?))
    }

    pub async fn update(
        context: &Context,
        id: Uuid,
        balance: RenderedBalanceUpdateInput,
    ) -> FieldResult<GraphQLBalance> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        require_own_balance(conn, user_id, id).await?;
        require_own_account(conn, user_id, balance.account_id).await?;

        diesel::update(balances::table.find(id))
            .set((
                balances::account_id.eq(balance.account_id),
                balances::date.eq(balance.date),
                balances::amount.eq(balance.amount),
                balances::comments.eq(balance.comments),
                balances::updated_at.eq(Utc::now()),
            ))
            .execute(conn)
            .await?;

        Ok(GraphQLBalance::new(load_rendered(conn, user_id, id).await?))
    }
}

type Conn = diesel_async::pooled_connection::deadpool::Object<diesel_async::AsyncPgConnection>;

async fn load_rendered(conn: &mut Conn, user_id: Uuid, id: Uuid) -> FieldResult<RenderedBalance> {
    balances_rendered::table
        .filter(balances_rendered::id.eq(id))
        .filter(balances_rendered::user_id.eq(user_id))
        .select(RenderedBalance::as_select())
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| format!("Balance {id} not found").into())
}

/// A mirrored account renders the other user's balances into this user's books, so owning the
/// rendering is not the same as owning the row behind it. Writes go to the upstream row, so
/// they have to be checked against the account that actually holds it.
async fn require_own_balance(conn: &mut Conn, user_id: Uuid, id: Uuid) -> FieldResult<()> {
    let owned: Option<Uuid> = balances::table
        .inner_join(accounts::table)
        .filter(balances::id.eq(id))
        .filter(accounts::user_id.eq(user_id))
        .select(balances::id)
        .first(conn)
        .await
        .optional()?;

    owned
        .map(|_| ())
        .ok_or_else(|| format!("Balance {id} not found").into())
}

async fn require_own_account(conn: &mut Conn, user_id: Uuid, account_id: Uuid) -> FieldResult<()> {
    let owner: Option<Uuid> = accounts::table
        .find(account_id)
        .select(accounts::user_id)
        .first(conn)
        .await
        .optional()?;

    match owner {
        Some(owner) if owner == user_id => Ok(()),
        Some(_) => Err(format!("Account {account_id} belongs to somebody else").into()),
        None => Err(format!("Account {account_id} does not exist").into()),
    }
}
