use std::{collections::HashMap, error::Error};

use crate::{
    models::{ObjectIdWrapper, User},
    queries, verify_password,
};

use dataloader::{cached::Loader, BatchFn};
use futures::{StreamExt, TryStreamExt};
use juniper::graphql_object;
use mongodb::{
    bson::{doc, oid::ObjectId},
    Database,
};

use super::root::{require_user, Context};

#[derive(Debug, Clone)]
pub struct GraphQLUserInfo {
    data: User,
}

impl GraphQLUserInfo {
    pub fn new(user: User) -> Self {
        Self { data: user }
    }
}

#[graphql_object(Context = Context, name = "User")]
impl GraphQLUserInfo {
    fn id(&self) -> ObjectIdWrapper {
        self.data.id
    }
    fn name(&self) -> &str {
        &self.data.name
    }

    fn full_name(&self) -> &str {
        &self.data.full_name
    }
}

pub struct UserQuery;

#[graphql_object(
    Context = Context,
)]
impl UserQuery {
    async fn me(context: &Context) -> juniper::FieldResult<GraphQLUserInfo> {
        let user_id = require_user(context).await?;

        let user = queries::query_element_by_id(&context.connection, "users", user_id)
            .await?
            .ok_or_else(|| {
                juniper::FieldError::new("User does not exist anymore", juniper::Value::null())
            })?;

        Ok(GraphQLUserInfo::new(user))
    }

    async fn get(context: &Context, id: ObjectIdWrapper) -> juniper::FieldResult<GraphQLUserInfo> {
        require_user(context).await?;
        Ok(GraphQLUserInfo::new(
            context.user_loader.try_load(id).await?,
        ))
    }

    async fn list(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
    ) -> juniper::FieldResult<Vec<GraphQLUserInfo>> {
        let collection = &context.connection.collection::<User>("users");
        let cursor = collection
            .find(None, None)
            .await?
            .skip(usize::try_from(skip.unwrap_or(0))?)
            .take(usize::try_from(limit.unwrap_or(i32::MAX))?);

        Ok(cursor
            .and_then(|u| async { Ok(GraphQLUserInfo::new(u)) })
            .try_collect::<Vec<_>>()
            .await?)
    }
}

pub struct UserMutation;

#[graphql_object(
    Context = Context,
)]
impl UserMutation {
    async fn login(
        context: &Context,
        name: String,
        password: String,
    ) -> juniper::FieldResult<GraphQLUserInfo> {
        if let Some(user) = queries::query_user_by_name(&context.connection, &name).await? {
            if verify_password(&password, &user.password_hash) {
                info!("Successful login with username '{}'", &name);
                let mut user_id = context.user_id.write().await;
                *user_id = Some(user.id.0);
                Ok(GraphQLUserInfo::new(user))
            } else {
                warn!("Attempt to login with wrong password for user '{}'", &name);
                Err(juniper::FieldError::new(
                    "Unauthorized",
                    juniper::Value::null(),
                ))
            }
        } else {
            warn!("Attempt to login with non-existing username '{}'", &name);
            Err(juniper::FieldError::new(
                "Unauthorized",
                juniper::Value::null(),
            ))
        }
    }

    async fn logout(context: &Context) -> juniper::FieldResult<bool> {
        let mut user_id = context.user_id.write().await;
        *user_id = None;
        Ok(true)
    }
}

pub async fn get_many(
    connection: &Database,
    ids: &[ObjectIdWrapper],
) -> Result<HashMap<ObjectIdWrapper, User>, Box<dyn Error>> {
    debug!(
        "Performing a coalesced category query for {} ids",
        ids.len()
    );

    let collection = connection.collection::<User>("users");
    let users = collection
        .find(
            doc! {"_id": {"$in": ids.iter().map(|i| **i).collect::<Vec<ObjectId>>()}},
            None,
        )
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    Ok(users
        .into_iter()
        .map(|a| (a.id, a))
        .collect::<HashMap<_, _>>())
}

pub struct UserBatcher {
    connection: Database,
}

#[async_trait]
impl BatchFn<ObjectIdWrapper, User> for UserBatcher {
    async fn load(&mut self, keys: &[ObjectIdWrapper]) -> HashMap<ObjectIdWrapper, User> {
        get_many(&self.connection, keys)
            .await
            .unwrap_or(HashMap::new())
    }
}

pub type UserLoader = Loader<ObjectIdWrapper, User, UserBatcher>;

pub fn get_user_loader(connection: Database) -> UserLoader {
    Loader::new(UserBatcher { connection })
}
