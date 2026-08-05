//! Account balances computed forward from the last reference balance.
//!
//! This is the one place raw SQL earns its keep: "newest balance per account, plus every
//! transaction since" is a `DISTINCT ON` and an aggregate, which the mongo version had to do
//! by pulling the expenses over the wire and folding them in Rust.

use uuid::Uuid;

use crate::graphql::root::{Context, require_user};
use crate::models::rendering::RenderedBalance;
use crate::schema_views::balances_rendered;

use chrono::DateTime;
use chrono::Utc;
use diesel::prelude::*;
use diesel::sql_types::{Double, Nullable, Timestamptz};
use diesel_async::RunQueryDsl;
use itertools::Itertools;
use juniper::{FieldResult, GraphQLObject, graphql_object};

use super::accounts::GraphQLAccount;
use super::balances::GraphQLBalance;

#[derive(Debug, Clone, GraphQLObject)]
#[graphql(context = Context)]
pub struct ComputedBalance {
    pub date: DateTime<Utc>,
    pub amount: f64,
    pub reference: GraphQLBalance,
    pub account: GraphQLAccount,
    pub expense_count: i32,
}

#[derive(Debug, QueryableByName)]
struct ComputedRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    balance_id: Uuid,
    #[diesel(sql_type = Double)]
    amount: f64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    expense_count: i64,
}

/// `$1` is the user, `$2` the cutoff date, `$3` an optional single account to restrict to.
///
/// The count is of expenses, not transactions: an expense booking twice against the same
/// account is one thing that happened.
const COMPUTE_BALANCES: &str = r#"
WITH newest AS (
    SELECT DISTINCT ON (account_id) id, account_id, date, amount
    FROM balances_rendered
    WHERE user_id = $1 AND date <= $2 AND ($3::uuid IS NULL OR account_id = $3)
    ORDER BY account_id, date DESC
)
SELECT
    n.id AS balance_id,
    n.amount::double precision + coalesce(sum(t.calculated_amount), 0) AS amount,
    count(DISTINCT t.expense_id) AS expense_count
FROM newest n
LEFT JOIN expense_transactions_rendered t
    ON t.user_id = $1
   AND t.account_id = n.account_id
   AND t.date >= n.date
   AND t.date < $2
   AND t.calculated_amount <> 0
GROUP BY n.id, n.amount
"#;

pub struct AnalysisQuery;

#[graphql_object(
    Context = Context,
)]
impl AnalysisQuery {
    pub async fn balance(
        context: &Context,
        date: Option<DateTime<Utc>>,
        account_id: Uuid,
    ) -> FieldResult<ComputedBalance> {
        let date = date.unwrap_or_else(Utc::now);
        let user_id = require_user(context).await?;

        let account = context.account_loader.try_load(account_id).await?;
        if account.user_id != user_id {
            return Err(format!(
                "Not going to tell you balances for account {} that isn't yours!",
                account.id
            )
            .into());
        }

        compute(context, date, Some(account_id))
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| format!("Account {account_id} has no balance to compute from").into())
    }

    pub async fn balances(
        context: &Context,
        date: Option<DateTime<Utc>>,
    ) -> FieldResult<Vec<ComputedBalance>> {
        let date = date.unwrap_or_else(Utc::now);
        require_user(context).await?;

        Ok(compute(context, date, None)
            .await?
            .into_iter()
            .sorted_by_key(|b| b.account.data.name.clone())
            .collect())
    }
}

async fn compute(
    context: &Context,
    date: DateTime<Utc>,
    account_id: Option<Uuid>,
) -> FieldResult<Vec<ComputedBalance>> {
    let user_id = require_user(context).await?;
    let conn = &mut context.conn().await?;

    let rows: Vec<ComputedRow> = diesel::sql_query(COMPUTE_BALANCES)
        .bind::<diesel::sql_types::Uuid, _>(user_id)
        .bind::<Timestamptz, _>(date)
        .bind::<Nullable<diesel::sql_types::Uuid>, _>(account_id)
        .load(conn)
        .await?;

    let references: Vec<RenderedBalance> = balances_rendered::table
        .filter(balances_rendered::user_id.eq(user_id))
        .filter(balances_rendered::id.eq_any(rows.iter().map(|r| r.balance_id).collect::<Vec<_>>()))
        .select(RenderedBalance::as_select())
        .load(conn)
        .await?;

    let accounts = context
        .account_loader
        .load_many(references.iter().map(|b| b.account_id).unique().collect())
        .await;

    let mut computed = Vec::with_capacity(references.len());
    for reference in references {
        let row = rows
            .iter()
            .find(|r| r.balance_id == reference.id)
            .expect("every reference balance came out of the computation");
        let account = accounts
            .get(&reference.account_id)
            .ok_or_else(|| format!("Account {} does not exist", reference.account_id))?;

        computed.push(ComputedBalance {
            date,
            amount: row.amount,
            expense_count: row.expense_count as i32,
            reference: GraphQLBalance::new(reference.clone()),
            account: GraphQLAccount::new(account.clone()),
        });
    }

    Ok(computed)
}
