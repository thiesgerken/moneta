use std::collections::HashMap;
use std::error::Error;

use crate::graphql::root::require_user;
use crate::graphql::root::Context;
use crate::graphql::util::{self};
use crate::models::rendering::{RenderedBalance, RenderedExpense};
use crate::models::AccountAvailability;
use crate::models::AccountKind;
use crate::models::AccountRisk;
use crate::models::AccountSynchronization;
use crate::models::{Account, AccountCreateInput, AccountUpdateInput, ObjectIdWrapper};

use chrono::DateTime;
use chrono::Utc;
use dataloader::cached::Loader;
use dataloader::BatchFn;
use futures::TryStreamExt;
use juniper::graphql_object;
use juniper::FieldResult;
use log::debug;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{self, doc};
use mongodb::Database;

use super::users::GraphQLUserInfo;

#[derive(Debug, Clone)]
pub struct GraphQLAccount {
    data: Account,
}
impl GraphQLAccount {
    pub fn new(data: Account) -> Self {
        Self { data }
    }
}

#[graphql_object(    Context = Context,name = "Account")]
impl GraphQLAccount {
    fn id(&self) -> ObjectIdWrapper {
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

    fn synchronization(&self) -> &Option<AccountSynchronization> {
        &self.data.synchronization
    }
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
        filter: Option<String>,
        sort: Option<String>,
    ) -> juniper::FieldResult<Vec<GraphQLAccount>> {
        require_user(context).await?;

        let accounts = util::list_query::<Account>(
            "accounts",
            &context.connection,
            limit,
            skip,
            filter,
            sort,
            None,
        )
        .await?
        .1;

        context
            .account_loader
            .prime_many(accounts.iter().map(|a| (a.id, a.clone()))).await;

        Ok(accounts.into_iter().map(GraphQLAccount::new).collect())
    }

    pub async fn get(
        context: &Context,
        id: ObjectIdWrapper,
    ) -> juniper::FieldResult<GraphQLAccount> {
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
            .count_documents(doc!("account_id": *account.id), None)
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
            .count_documents(doc!("account_id": *account.id), None)
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
            .await?
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

pub async fn get_many(
    connection: &Database,
    ids: &[ObjectIdWrapper],
) -> Result<HashMap<ObjectIdWrapper, Account>, Box<dyn Error>> {
    debug!("Performing a coalesced account query for {} ids", ids.len());

    let collection = connection.collection::<Account>("accounts");
    let accounts = collection
        .find(
            doc! {"_id": {"$in": ids.iter().map(|i| **i).collect::<Vec<ObjectId>>()}},
            None,
        )
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    Ok(accounts
        .into_iter()
        .map(|a| (a.id, a))
        .collect::<HashMap<_, _>>())
}

pub struct AccountBatcher {
    connection: Database,
}

#[async_trait]
impl BatchFn<ObjectIdWrapper, Account> for AccountBatcher {
    async fn load(&mut self, keys: &[ObjectIdWrapper]) -> HashMap<ObjectIdWrapper, Account> {
        get_many(&self.connection, keys).await.unwrap_or_default()
    }
}

pub type AccountLoader = Loader<ObjectIdWrapper, Account, AccountBatcher>;

pub fn get_account_loader(connection: Database) -> AccountLoader {
    Loader::new(AccountBatcher { connection })
}
