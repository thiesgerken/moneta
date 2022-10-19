use std::str::FromStr;

use crate::models::rendering::{RenderedBalance, RenderedExpense};
use crate::models::{Account, AccountCreateInput, AccountUpdateInput, ObjectIdWrapper};
use crate::web::authentication::CookieUserId;
use crate::web::graphql::require_user;
use crate::web::graphql::Context;
use crate::web::util::{self, log_error_and_500};

use juniper::graphql_object;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{self, doc};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;

pub struct AccountQuery;

#[graphql_object(
    Context = Context,
)]
impl AccountQuery {
    pub async fn list(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
        filter: Option<String>,
        sort: Option<String>,
    ) -> juniper::FieldResult<Vec<Account>> {
        require_user(context).await?;

        Ok(util::list_query::<Account>(
            "accounts",
            &context.connection,
            limit,
            skip,
            filter,
            sort,
            None,
        )
        .await?)
    }

    pub async fn get(context: &Context, id: ObjectIdWrapper) -> juniper::FieldResult<Account> {
        require_user(context).await?;
        let collection = context.connection.collection::<Account>("accounts");

        let entry = collection.find_one(doc! {"_id":*id}, None).await?;

        if let Some(entry) = entry {
            Ok(entry)
        } else {
            Err(juniper::FieldError::new(
                "Account does not exist",
                juniper::Value::null(),
            ))
        }
    }
}

pub struct AccountMutation;

#[graphql_object(
    Context = Context,
)]
impl AccountMutation {
    pub async fn delete(context: &Context, id: ObjectIdWrapper) -> juniper::FieldResult<bool> {
        let user_id = require_user(context).await?;
        let collection = context.connection.collection::<Account>("accounts");

        let account = collection
            .find_one(doc! {"_id":*id, "userId": user_id}, None)
            .await?
            .ok_or("entry to delete not found")?;

        if account.synchronization.is_some() {
            return Err(format!(
                "Deletion of synchronized accounts is unsupported, tried to remove {:?}",
                id
            )
            .into());
        }

        // check that no rendered balances reference this entry
        let rendered_balances_collection = context
            .connection
            .collection::<RenderedBalance>("balances_rendered");
        let balance_count = rendered_balances_collection
            .count_documents(doc!("account_id": *account.id.clone()), None)
            .await?;

        if balance_count > 0 {
            return Err(format!(
                "Tried to remove account {:?}, but it still has {} (rendered) balances attached to it",
                id, balance_count
            ).into());
        }

        // check that no rendered expenses reference this entry
        let rendered_expenses_collection = context
            .connection
            .collection::<RenderedExpense>("expenses_rendered");
        let expense_count = rendered_expenses_collection
            .count_documents(doc!("account_id": *account.id.clone()), None)
            .await?;
        if expense_count > 0 {
            return Err( format!(
                "Tried to remove account {:?}, but it still has {} (rendered) expenses attached to it",
                id, expense_count
            ).into());
        }

        // everything seems to be in order, proceed with the deletion
        let count = collection
            .delete_one(doc! {"_id":*id, "userId": user_id}, None)
            .await?
            .deleted_count;
        Ok(count == 1)
    }

    pub async fn create(
        context: &Context,
        account: AccountCreateInput,
    ) -> juniper::FieldResult<Account> {
        let user_id = require_user(context).await?;
        let collection = context.connection.collection::<Account>("accounts");

        account.validate()?;

        let account = account.upgrade(ObjectId::new().into(), user_id.into());
        collection.insert_one(&account, None).await?;

        Ok(account)
    }

    pub async fn update(
        context: &Context,
        id: ObjectIdWrapper,
        account: AccountUpdateInput,
    ) -> juniper::FieldResult<bool> {
        let user_id = require_user(context).await?;
        let collection = context.connection.collection::<Account>("accounts");

        collection
            .find_one(doc! {"_id":*id, "userId": user_id}, None)
            .await
            .map_err(|e| log_error_and_500(Box::new(e)))?
            .ok_or("entry to update not found")?;

        account.validate()?;

        let count = collection
            .update_one(
                doc! {"_id":*id,"userId": user_id},
                doc! {"$set":  bson::to_bson(&account).unwrap()},
                None,
            )
            .await?
            .modified_count;

        Ok(count == 1)
    }
}
