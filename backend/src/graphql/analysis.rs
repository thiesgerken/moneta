use std::collections::HashMap;

use crate::graphql::root::Context;
use crate::graphql::root::require_user;
use crate::models::Uuid;
use crate::models::rendering::{RenderedBalance, RenderedExpense};

use chrono::DateTime;
use chrono::Utc;
use itertools::Itertools;
use juniper::GraphQLObject;
use juniper::graphql_object;

use mongodb::bson::doc;

use super::accounts::GraphQLAccount;
use super::balances::GraphQLBalance;
use uuid::Uuid;

#[derive(Debug, Clone, GraphQLObject)]
#[graphql(context = Context)]
pub struct ComputedBalance {
    pub date: DateTime<Utc>,
    pub amount: f64,
    pub reference: GraphQLBalance,
    pub account: GraphQLAccount,
    pub expense_count: i32,
}

pub struct AnalysisQuery;

#[graphql_object(
    Context = Context,
)]
impl AnalysisQuery {
    pub async fn balance(
        context: &Context,
        date: Option<DateTime<Utc>>,
        account_id: Uuid,
    ) -> juniper::FieldResult<ComputedBalance> {
        let date = date.unwrap_or(Utc::now());
        let user = require_user(context).await?;
        let account = context.account_loader.try_load(account_id).await?;

        if *account.user_id != user {
            return Err(format!(
                "Not going to tell you balances for account {:?} that isn't yours!",
                account.id
            )
            .into());
        }

        let balance_collection = context
            .connection
            .collection::<RenderedBalance>("balances_rendered");
        let newest_balance = balance_collection
            .find_one(doc! {"accountId": *account_id, "userId": user , "date": {"$lte": date} })
            .sort(doc! {"date": -1})
            .await?
            .unwrap(); // we can panic here

        let expense_collection = context
            .connection
            .collection::<RenderedExpense>("expenses_rendered");
        let mut expense_cursor =        expense_collection
        .find(
            doc! { "userId": user,"transactions" : {"$elemMatch": {"accountId": *account_id , "date": {"$lt": date,"$gte": newest_balance.date}}}}
        )
        .await?;

        let mut computed_amount = newest_balance.amount as f64;
        let mut expense_count = 0;
        while let Some(exp) = expense_cursor.try_next().await? {
            // need to filter again also for the date; the expense might have multiple transactions with this account and only some might match the date filter
            let contribution = exp
                .transactions
                .iter()
                .filter(|t| {
                    t.account_id == account_id && t.date < date && t.date >= newest_balance.date
                })
                .fold(0.0, |acc, t| acc + t.calculated_amount);

            if contribution != 0.0 {
                computed_amount += contribution;
                expense_count += 1;
            }
        }

        Ok(ComputedBalance {
            date,
            amount: computed_amount,
            reference: GraphQLBalance::new(newest_balance),
            account: GraphQLAccount::new(account),
            expense_count,
        })
    }

    pub async fn balances(
        context: &Context,
        date: Option<DateTime<Utc>>,
    ) -> juniper::FieldResult<Vec<ComputedBalance>> {
        let date = date.unwrap_or(Utc::now());
        let user = require_user(context).await?;

        let balance_collection = context
            .connection
            .collection::<RenderedBalance>("balances_rendered");

        // one could also do an aggregation here, but we can also simply query all balances; shouldn't be that many
        let mut newest_balances = balance_collection
            .find(doc! { "userId": user , "date": {"$lte": date} })
            .sort(doc! {"date": 1})
            .await?
            .and_then(|b| async { Ok((b.account_id, b)) })
            .try_collect::<HashMap<_, _>>()
            .await?;

        let accounts = context
            .account_loader
            .load_many(newest_balances.keys().copied().collect::<Vec<_>>())
            .await;

        let expense_collection = context
            .connection
            .collection::<RenderedExpense>("expenses_rendered");

        // NOTE: in production builds, it is faster to query a few more expenses and filter in here
        //       than to let mongo do it. For reference & experiments, here is the other version:
        // let filters: Vec<Document> = newest_balances
        //     .iter()
        //     .map(|(_,b)| doc!{ "transactions": {"$elemMatch" : {"date": {"$gte": b.date, "$lt": date}, "accountId": *b.account_id }}})
        //     .collect();
        // let mut expense_cursor = expense_collection
        //     .find(doc! { "userId": user,"$or" : filters}, None)
        //     .await?;

        let oldest_balance_date = newest_balances
            .values()
            .map(|b| b.date)
            .min()
            .unwrap_or(date);
        let mut expense_cursor = expense_collection
            .find(
                doc! { "userId": user,"transactions" : {"$elemMatch": { "date": {"$lt": date,"$gte": oldest_balance_date}}}}
            )
            .await?;

        let mut computed_amounts = newest_balances
            .iter()
            .map(|(k, v)| (*k, v.amount as f64))
            .collect::<HashMap<_, _>>();
        let mut expense_counts = newest_balances
            .keys()
            .map(|k| (*k, 0))
            .collect::<HashMap<_, _>>();
        while let Some(exp) = expense_cursor.try_next().await? {
            for transaction in exp
                .transactions
                .iter()
                .filter(|t| t.calculated_amount != 0.0 && t.date < date)
            {
                if let Some(newest_balance) = newest_balances.get(&transaction.account_id)
                    && transaction.date >= newest_balance.date
                {
                    let computed_amount =
                        computed_amounts.get_mut(&transaction.account_id).unwrap();
                    let expense_count = expense_counts.get_mut(&transaction.account_id).unwrap();

                    *computed_amount += transaction.calculated_amount;
                    *expense_count += 1;
                }
            }
        }

        Ok(accounts
            .into_iter()
            .map(|(k, v)| ComputedBalance {
                date,
                amount: computed_amounts.remove(&k).unwrap(),
                reference: GraphQLBalance::new(newest_balances.remove(&k).unwrap()),
                account: GraphQLAccount::new(v),
                expense_count: expense_counts.remove(&k).unwrap(),
            })
            .sorted_by_key(|b| b.account.data.name.clone())
            .collect())
    }
}
