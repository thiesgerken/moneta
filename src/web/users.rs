use crate::{
    models::{ObjectIdWrapper, User},
    queries, verify_password,
};

use futures::{StreamExt, TryStreamExt};
use juniper::{graphql_object, GraphQLObject};
use mongodb::bson::doc;
use serde::Serialize;

use super::graphql::{require_user, Context};

#[derive(Serialize, GraphQLObject)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    #[serde(rename = "_id")]
    id: ObjectIdWrapper,
    name: String,
    full_name: String,
}

impl UserInfo {
    pub fn new(user: User) -> Self {
        Self {
            id: user.id,
            name: user.name,
            full_name: user.full_name,
        }
    }
}

pub struct UserQuery;

#[graphql_object(
    Context = Context,
)]
impl UserQuery {
    async fn me(context: &Context) -> juniper::FieldResult<UserInfo> {
        let user_id = require_user(context).await?;

        let user = queries::query_element_by_id(&context.connection, "users", user_id)
            .await?
            .ok_or_else(|| {
                juniper::FieldError::new("User does not exist anymore", juniper::Value::null())
            })?;

        Ok(UserInfo::new(user))
    }

    async fn list(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
    ) -> juniper::FieldResult<Vec<UserInfo>> {
        let collection = &context.connection.collection::<User>("users");
        let cursor = collection
            .find(None, None)
            .await?
            .skip(usize::try_from(skip.unwrap_or(0))?)
            .take(usize::try_from(limit.unwrap_or(i32::MAX))?);

        Ok(cursor
            .and_then(|u| async { Ok(UserInfo::new(u)) })
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
    ) -> juniper::FieldResult<UserInfo> {
        if let Some(user) = queries::query_user_by_name(&context.connection, &name).await? {
            if verify_password(&password, &user.password_hash) {
                info!("Successful login with username '{}'", &name);
                let mut user_id = context.user_id.write().await;
                *user_id = Some(user.id.0);
                Ok(UserInfo::new(user))
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
