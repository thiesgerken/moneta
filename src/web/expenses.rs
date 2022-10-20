use std::error::Error;

use crate::models::rendering::{
    RenderedExpense, RenderedExpenseCreateInput, RenderedExpenseInput, RenderedExpenseUpdateInput,
};
use crate::models::{Account, Expense, ObjectIdWrapper};
use crate::queries;
use crate::web::graphql::require_user;
use crate::web::graphql::Context;
use crate::web::util::{self};

use juniper::graphql_object;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{self, doc};

pub struct ExpenseQuery;

#[graphql_object(
    Context = Context,
)]
impl ExpenseQuery {
    pub async fn list(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
        filter: Option<String>,
        sort: Option<String>,
    ) -> juniper::FieldResult<Vec<RenderedExpense>> {
        let uid = require_user(context).await?;

        Ok(util::list_query::<RenderedExpense>(
            "expenses_rendered",
            &context.connection,
            limit,
            skip,
            filter,
            sort,
            Some(doc!("userId": uid)),
        )
        .await?)
    }

    pub async fn get(
        context: &Context,
        upstream_id: ObjectIdWrapper,
    ) -> juniper::FieldResult<RenderedExpense> {
        let uid = require_user(context).await?;
        let collection = context
            .connection
            .collection::<RenderedExpense>("expenses_rendered");

        let entry = collection
            .find_one(doc! {"upstreamId":*upstream_id, "userId": uid}, None)
            .await?;

        if let Some(entry) = entry {
            Ok(entry)
        } else {
            Err(juniper::FieldError::new(
                "RenderedExpense does not exist",
                juniper::Value::null(),
            ))
        }
    }
}
pub struct ExpenseMutation;

#[graphql_object(
    Context = Context,
)]
impl ExpenseMutation {
    pub async fn delete(context: &Context, id: ObjectIdWrapper) -> juniper::FieldResult<bool> {
        let user_id = require_user(context).await?;
        let collection = context
            .connection
            .collection::<RenderedExpense>("expenses_rendered");

        // check if there is a rendered expense with the given upstream id belonging to this user
        let original_entry = collection
            .find_one(doc! {"_id":*id, "userId": user_id}, None)
            .await?
            .ok_or("entry to delete not found")?;

        let upstream_collection = context.connection.collection::<Expense>("expenses");
        let upstream_entry = upstream_collection
            .find_one(doc! {"_id":*original_entry.upstream_id}, None)
            .await?
            .unwrap_or_else(|| {
                panic!("cannot find upstream expense for rendered expense {:?}", id)
            });

        // remove rendered expense first ...
        upstream_entry.remove_rendered(&context.connection).await?;
        // ... and then the upstream expense itself
        let count = upstream_collection
            .delete_one(doc! {"_id":*original_entry.upstream_id}, None)
            .await?
            .deleted_count;
        Ok(count == 1)
    }

    pub async fn create(
        context: &Context,
        expense: RenderedExpenseCreateInput,
    ) -> juniper::FieldResult<RenderedExpense> {
        let user_id = require_user(context).await?;
        let collection = context.connection.collection::<Expense>("expenses");

        let occurring_account_ids: Vec<_> =
            expense.transactions.iter().map(|t| *t.account_id).collect();
        let related_accounts =
            queries::query_associated_accounts(&context.connection, &occurring_account_ids).await?;
        let entry = validate_and_unrender(user_id, expense, &related_accounts).await?;

        collection.insert_one(&entry, None).await?;

        let expense = render(user_id, &context.connection, entry, &related_accounts).await?;
        Ok(expense)
    }

    pub async fn update(
        context: &Context,
        id: ObjectIdWrapper,
        expense: RenderedExpenseUpdateInput,
    ) -> juniper::FieldResult<RenderedExpense> {
        let user_id = require_user(context).await?;
        let collection = context
            .connection
            .collection::<RenderedExpense>("expenses_rendered");

        let original_rendered_expense = collection
            .find_one(doc! {"_id":*id, "userId": user_id}, None)
            .await?
            .ok_or("entry to update not found")?;

        let occurring_account_ids: Vec<_> =
            expense.transactions.iter().map(|t| *t.account_id).collect();
        let related_accounts =
            queries::query_associated_accounts(&context.connection, &occurring_account_ids).await?;

        let mut updated_upstream_entry =
            validate_and_unrender(user_id, expense, &related_accounts).await?;

        // find the original upstream expense
        let upstream_collection = context.connection.collection::<Expense>("expenses");
        let upstream_entry = upstream_collection
            .find_one(doc!("_id": *original_rendered_expense.upstream_id), None)
            .await?
            .unwrap_or_else(|| {
                panic!("cannot find upstream expense for rendered expense {:?}", id)
            });

        // delete its rendered expenses
        upstream_entry.remove_rendered(&context.connection).await?;

        // update the upstream expense with the changes
        updated_upstream_entry.id = upstream_entry.id;
        upstream_collection
            .update_one(
                doc!("_id": &*updated_upstream_entry.id),
                doc! {"$set":  bson::to_bson(&updated_upstream_entry).unwrap()},
                None,
            )
            .await?;

        let expense = render(
            user_id,
            &context.connection,
            updated_upstream_entry,
            &related_accounts,
        )
        .await?;
        Ok(expense)
    }
}

pub async fn validate_and_unrender(
    uid: ObjectId,
    expense: RenderedExpenseInput,
    related_accounts: &[Account],
) -> Result<Expense, Box<dyn Error>> {
    if expense
        .transactions
        .iter()
        .map(|t| *t.account_id)
        .any(|a_id| !related_accounts.iter().any(|a| *a.id == a_id))
    {
        return Err(format!(
            "user {:?} tried to create an expense with unknown account id(s)",
            uid
        )
        .into());
    };

    if related_accounts.iter().all(|a| *a.user_id != uid) {
        return Err(format!(
            "user {:?} tried to create an expense consisting only of accounts that don't belong to her",
            uid
        ).into());
    }

    if expense
        .categories
        .iter()
        .map(|c| c.category_id.clone())
        .any(|a_id| !related_accounts.iter().any(|a| *a.id == *a_id))
    {
        return Err(format!(
            "user {:?} tried to create an expense with unknown category id(s)",
            uid
        )
        .into());
    };

    // cast to normal expense and insert into database
    let entry = Expense::from_rendered(expense);
    entry.validate()?;

    Ok(entry)
}

pub async fn render(
    uid: ObjectId,
    connection: &mongodb::Database,
    entry: Expense,
    related_accounts: &[Account],
) -> Result<RenderedExpense, Box<dyn Error>> {
    // create rendered expenses and insert them into database
    let rendered_expenses = entry.render(related_accounts)?;
    let collection = connection.collection::<RenderedExpense>("expenses_rendered");
    collection.insert_many(&rendered_expenses, None).await?;

    // find the rendered expense that was rendered for this user and return it
    let rendered_entry = rendered_expenses
        .into_iter()
        .find(|b| *b.user_id == uid)
        .unwrap_or_else(|| {
            panic!(
                "rendering did not result in a expense for this user?! expense {:?}",
                entry.id
            )
        });
    Ok(rendered_entry)
}
