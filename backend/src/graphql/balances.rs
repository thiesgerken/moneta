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
use diesel::sql_types::Bool;
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
            context
                .account_loader
                .try_load(self.data.account_id)
                .await?,
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
    Comments,
    Account,
}

type BalancePredicate =
    Box<dyn BoxableExpression<balances_rendered::table, diesel::pg::Pg, SqlType = Bool>>;

/// The comments, plus -- if the term reads as a number -- the amount in either sign, since a
/// balance can be negative and the table shows it unsigned.
fn search_predicate(term: &str) -> BalancePredicate {
    let mut predicate: BalancePredicate =
        Box::new(balances_rendered::comments.ilike(util::ilike_pattern(term)));

    if let Ok(value) = term.replace(',', ".").parse::<f64>()
        && value != 0.0
    {
        let lower = (value * 100.0).floor() as i32;
        let upper = (value * 100.0).ceil() as i32;

        predicate = Box::new(
            predicate
                .or(balances_rendered::amount.between(lower, upper))
                .or(balances_rendered::amount.between(-upper, -lower)),
        );
    }

    predicate
}

pub struct BalanceQuery;

#[graphql_object(
    Context = Context,
)]
impl BalanceQuery {
    #[allow(clippy::too_many_arguments)]
    pub async fn list(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
        search: Option<String>,
        account_ids: Option<Vec<Uuid>>,
        after: Option<DateTime<Utc>>,
        before: Option<DateTime<Utc>>,
        sort_by: Option<BalanceSortField>,
        sort_dir: Option<SortDirection>,
    ) -> FieldResult<BalanceConnection> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let (limit, offset) = util::page(limit, skip);
        let direction = sort_dir.unwrap_or_default();
        let search = search.filter(|s| !s.trim().is_empty());

        // A boxed query is consumed by the count, so the filter is built twice rather than
        // cloned.
        let filtered = || {
            let mut query = balances_rendered::table
                .into_boxed()
                .filter(balances_rendered::user_id.eq(user_id));

            if let Some(term) = &search {
                query = query.filter(search_predicate(term.trim()));
            }
            if let Some(account_ids) = &account_ids
                && !account_ids.is_empty()
            {
                query = query.filter(balances_rendered::account_id.eq_any(account_ids));
            }
            if let Some(after) = after {
                query = query.filter(balances_rendered::date.ge(after));
            }
            if let Some(before) = before {
                query = query.filter(balances_rendered::date.le(before));
            }

            query
        };

        let total_count: i64 = filtered().count().get_result(conn).await?;

        let query = filtered();
        let query = match sort_by.unwrap_or_default() {
            BalanceSortField::Date => order_by!(query, direction, balances_rendered::date),
            BalanceSortField::Amount => order_by!(query, direction, balances_rendered::amount),
            BalanceSortField::Comments => order_by!(query, direction, balances_rendered::comments),
            BalanceSortField::Account => order_by!(query, direction, balances_rendered::account_id),
        };

        let found = query
            // without a tiebreaker, two pages of an equal sort key can repeat or skip rows
            .then_order_by(balances_rendered::id.asc())
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

        let count = diesel::delete(balances::table.find(id))
            .execute(conn)
            .await?;
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
