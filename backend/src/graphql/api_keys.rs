use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::root::{Context, require_session};
use crate::api_keys::{self, ApiKey, ApiKeyAccess, CreatedApiKey};

pub struct ApiKeyQuery;
pub struct ApiKeyMutation;

#[juniper::graphql_object(context = Context)]
impl ApiKeyQuery {
    /// Lists only this browser user's keys, including expired credentials. Session-only.
    async fn list(context: &Context) -> juniper::FieldResult<Vec<ApiKey>> {
        let user_id = require_session(context).await?;
        let conn = &mut context.conn().await?;
        Ok(api_keys::list(conn, user_id).await?)
    }
}

#[juniper::graphql_object(context = Context)]
impl ApiKeyMutation {
    /// Creates a fixed-permission credential. No expiry unless explicitly supplied. Session-only.
    async fn create(
        context: &Context,
        name: String,
        access: ApiKeyAccess,
        expires_at: Option<DateTime<Utc>>,
    ) -> juniper::FieldResult<CreatedApiKey> {
        let user_id = require_session(context).await?;
        let conn = &mut context.conn().await?;
        api_keys::create(conn, user_id, &name, access, expires_at).await
    }

    /// Physically deletes an owned key. Authentication fails on its next request. Session-only.
    async fn revoke(context: &Context, id: Uuid) -> juniper::FieldResult<bool> {
        let user_id = require_session(context).await?;
        let conn = &mut context.conn().await?;
        Ok(api_keys::revoke(conn, user_id, id).await?)
    }
}
