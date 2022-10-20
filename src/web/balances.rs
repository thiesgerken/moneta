use std::error::Error;

use crate::models::rendering::{
    RenderedBalance, RenderedBalanceCreateInput, RenderedBalanceInput, RenderedBalanceUpdateInput,
};
use crate::models::{Account, Balance, ObjectIdWrapper};
use crate::queries;
use crate::web::graphql::require_user;
use crate::web::graphql::Context;
use crate::web::util::{self};

use juniper::graphql_object;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{self, doc};

pub struct BalanceQuery;

#[graphql_object(
    Context = Context,
)]
impl BalanceQuery {
    pub async fn list(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
        filter: Option<String>,
        sort: Option<String>,
    ) -> juniper::FieldResult<Vec<RenderedBalance>> {
        let uid = require_user(context).await?;

        Ok(util::list_query::<RenderedBalance>(
            "balances_rendered",
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
    ) -> juniper::FieldResult<RenderedBalance> {
        let uid = require_user(context).await?;
        let collection = context
            .connection
            .collection::<RenderedBalance>("balances_rendered");

        let entry = collection
            .find_one(doc! {"upstreamId":*upstream_id, "userId": uid}, None)
            .await?;

        if let Some(entry) = entry {
            Ok(entry)
        } else {
            Err(juniper::FieldError::new(
                "Balance does not exist",
                juniper::Value::null(),
            ))
        }
    }
}

pub struct BalanceMutation;

#[graphql_object(
    Context = Context,
)]
impl BalanceMutation {
    pub async fn delete(context: &Context, id: ObjectIdWrapper) -> juniper::FieldResult<bool> {
        let user_id = require_user(context).await?;
        let collection = context
            .connection
            .collection::<RenderedBalance>("balances_rendered");

        // check if there is a rendered balance with the given upstream id belonging to this user
        let original_entry = collection
            .find_one(doc! {"_id":*id, "userId": user_id}, None)
            .await?
            .ok_or("entry to delete not found")?;

        let upstream_collection = context.connection.collection::<Balance>("balances");
        let upstream_entry = upstream_collection
            .find_one(doc! {"_id":*original_entry.upstream_id}, None)
            .await?
            .unwrap_or_else(|| {
                panic!("cannot find upstream balance for rendered balance {:?}", id)
            });

        // remove rendered balances first ...
        upstream_entry.remove_rendered(&context.connection).await?;
        // ... and then the upstream balance itself
        let count = upstream_collection
            .delete_one(doc! {"_id":*original_entry.upstream_id}, None)
            .await?
            .deleted_count;
        Ok(count == 1)
    }

    pub async fn create(
        context: &Context,
        balance: RenderedBalanceCreateInput,
    ) -> juniper::FieldResult<RenderedBalance> {
        let user_id = require_user(context).await?;
        let collection = context.connection.collection::<Balance>("balances");

        let related_accounts =
            queries::query_associated_accounts(&context.connection, &[*balance.account_id]).await?;
        let entry = validate_and_unrender(user_id, balance, &related_accounts).await?;

        collection.insert_one(&entry, None).await?;

        let balance = render(user_id, &context.connection, entry, &related_accounts).await?;
        Ok(balance)
    }

    pub async fn update(
        context: &Context,
        id: ObjectIdWrapper,
        balance: RenderedBalanceUpdateInput,
    ) -> juniper::FieldResult<RenderedBalance> {
        let user_id = require_user(context).await?;
        let collection = context
            .connection
            .collection::<RenderedBalance>("balances_rendered");

        let original_rendered_balance = collection
            .find_one(doc! {"_id":*id, "userId": user_id}, None)
            .await?
            .ok_or("entry to update not found")?;

        let related_accounts =
            queries::query_associated_accounts(&context.connection, &[*balance.account_id]).await?;
        let mut updated_upstream_entry =
            validate_and_unrender(user_id, balance, &related_accounts).await?;

        // find the original upstream balance
        let upstream_collection = context.connection.collection::<Balance>("balances");
        let upstream_entry = upstream_collection
            .find_one(doc!("_id": *original_rendered_balance.upstream_id), None)
            .await?
            .unwrap_or_else(|| {
                panic!("cannot find upstream balance for rendered balance {:?}", id)
            });

        // delete its rendered balances
        upstream_entry.remove_rendered(&context.connection).await?;

        // update the upstream balance with the changes
        updated_upstream_entry.id = upstream_entry.id;
        upstream_collection
            .update_one(
                doc!("_id": &*updated_upstream_entry.id),
                doc! {"$set":  bson::to_bson(&updated_upstream_entry).unwrap()},
                None,
            )
            .await?;

        let balance = render(
            user_id,
            &context.connection,
            updated_upstream_entry,
            &related_accounts,
        )
        .await?;
        Ok(balance)
    }
}

pub async fn render(
    uid: ObjectId,
    connection: &mongodb::Database,
    entry: Balance,
    related_accounts: &[Account],
) -> Result<RenderedBalance, Box<dyn Error>> {
    // create rendered balances and insert them into database
    let rendered_balances = entry.render(related_accounts)?;
    let collection = connection.collection::<RenderedBalance>("balances_rendered");
    collection.insert_many(&rendered_balances, None).await?;

    // find the rendered balance that was rendered for this user and return it
    let rendered_entry = rendered_balances
        .into_iter()
        .find(|b| *b.user_id == uid)
        .unwrap_or_else(|| {
            panic!(
                "rendering did not result in a balance for this user?! balance {:?}",
                entry.id
            )
        });

    Ok(rendered_entry)
}

pub async fn validate_and_unrender(
    uid: ObjectId,
    balance: RenderedBalanceInput,
    related_accounts: &[Account],
) -> Result<Balance, Box<dyn Error>> {
    let account = related_accounts
        .iter()
        .find(|a| a.id == balance.account_id)
        .ok_or_else(|| {
            format!(
                "user {:?} tried to create a balance with unknown account {:?}",
                uid, &balance.account_id
            )
        })?;

    if *account.user_id != uid {
        return Err(format!(
            "user {:?} tried to create a balance with foreign account {:?}",
            uid, &balance.account_id
        )
        .into());
    }

    Ok(Balance::from_rendered(balance))
}
