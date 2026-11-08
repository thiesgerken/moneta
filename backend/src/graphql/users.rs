use std::collections::HashMap;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::graphql::util;
use crate::schema::users;
use crate::{models::User, queries, verify_password};

use dataloader::{BatchFn, cached::Loader};
use juniper::graphql_object;

use super::root::{Context, require_user};

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
    fn id(&self) -> Uuid {
        self.data.id
    }
    fn name(&self) -> &str {
        &self.data.name
    }

    fn full_name(&self) -> &str {
        &self.data.full_name
    }

    fn first_name(&self) -> Option<&str> {
        if let Some((first, _)) = self.data.full_name.split_once(' ') {
            Some(first)
        } else {
            None
        }
    }

    fn last_name(&self) -> Option<&str> {
        if let Some((_, last)) = self.data.full_name.split_once(' ') {
            Some(last)
        } else {
            None
        }
    }
}

/// When the reader was last here, and when they were here before that.
///
/// `previous` is what a start page dates its news from: null on a first visit, which the
/// reader has no history to be told about anyway.
#[derive(Debug, juniper::GraphQLObject)]
#[graphql(context = Context)]
pub struct Visit {
    pub current: chrono::DateTime<chrono::Utc>,
    pub previous: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, diesel::QueryableByName)]
struct VisitRow {
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
    seen_at: Option<chrono::DateTime<chrono::Utc>>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
    seen_before: Option<chrono::DateTime<chrono::Utc>>,
}

/// How long a visit lasts.
///
/// Reloading the page, or coming back to the tab over lunch, is the same visit: the marker
/// stays put and the page keeps saying what it said, which is the point of a marker at all.
/// Coming back in the evening is a new one.
const VISIT_HOURS: i64 = 4;

/// One statement, so two tabs opening together cannot both decide they are the new visit and
/// roll the marker twice -- the second would land on the first's own arrival.
const VISIT: &str = r#"
UPDATE users
SET seen_before = CASE WHEN seen_at IS NULL OR seen_at < $2 THEN seen_at ELSE seen_before END,
    seen_at     = CASE WHEN seen_at IS NULL OR seen_at < $2 THEN now()   ELSE seen_at     END
WHERE id = $1
RETURNING seen_at, seen_before
"#;

pub struct UserQuery;

#[graphql_object(
    Context = Context,
)]
impl UserQuery {
    async fn me(context: &Context) -> juniper::FieldResult<GraphQLUserInfo> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let user = users::table
            .find(user_id)
            .select(User::as_select())
            .first(conn)
            .await
            .optional()?
            .ok_or_else(|| {
                juniper::FieldError::new("User does not exist anymore", juniper::Value::null())
            })?;

        Ok(GraphQLUserInfo::new(user))
    }

    async fn get(context: &Context, id: Uuid) -> juniper::FieldResult<GraphQLUserInfo> {
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
        require_user(context).await?;
        let conn = &mut context.conn().await?;

        let (limit, offset) = util::page(limit, skip);
        let found = users::table
            .order_by(users::name.asc())
            .limit(limit)
            .offset(offset)
            .select(User::as_select())
            .load(conn)
            .await?;

        context
            .user_loader
            .prime_many(found.iter().map(|a| (a.id, a.clone())))
            .await;

        Ok(found.into_iter().map(GraphQLUserInfo::new).collect())
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
        let conn = &mut context.conn().await?;

        if let Some(user) = queries::query_user_by_name(conn, &name).await? {
            let Some(hash) = &user.password_hash else {
                warn!(
                    "Attempt to login with a password for user '{}', who has none",
                    &name
                );
                return Err(juniper::FieldError::new(
                    "Unauthorized",
                    juniper::Value::null(),
                ));
            };

            if verify_password(&password, hash) {
                info!("Successful login with username '{}'", &name);
                let mut user_id = context.user_id.write().await;
                *user_id = Some(user.id);
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

    /// Records that the reader is here, and says when they were here before.
    ///
    /// A mutation because it writes, but it is what the start page opens with: it needs the
    /// previous visit to date its news from, and that answer stops being available the moment
    /// this one is recorded.
    async fn visit(context: &Context) -> juniper::FieldResult<Visit> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let now = chrono::Utc::now();
        let row: VisitRow = diesel::sql_query(VISIT)
            .bind::<diesel::sql_types::Uuid, _>(user_id)
            .bind::<diesel::sql_types::Timestamptz, _>(now - chrono::Duration::hours(VISIT_HOURS))
            .get_result(conn)
            .await?;

        Ok(Visit {
            current: row.seen_at.unwrap_or(now),
            previous: row.seen_before,
        })
    }

    async fn logout(context: &Context) -> juniper::FieldResult<bool> {
        let mut user_id = context.user_id.write().await;
        *user_id = None;
        Ok(true)
    }
}

pub struct UserBatcher {
    pool: crate::Pool,
}

impl BatchFn<Uuid, User> for UserBatcher {
    async fn load(&mut self, keys: &[Uuid]) -> HashMap<Uuid, User> {
        debug!("Performing a coalesced user query for {} ids", keys.len());

        let Ok(mut conn) = self.pool.get().await else {
            return HashMap::new();
        };

        users::table
            .filter(users::id.eq_any(keys))
            .select(User::as_select())
            .load(&mut conn)
            .await
            .map(|found| found.into_iter().map(|u| (u.id, u)).collect())
            .unwrap_or_default()
    }
}

pub type UserLoader = Loader<Uuid, User, UserBatcher>;

pub fn get_user_loader(pool: crate::Pool) -> UserLoader {
    Loader::new(UserBatcher { pool })
}
