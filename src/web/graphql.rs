use std::sync::Arc;

use juniper::graphql_object;
use log::info;
use mongodb::bson::oid::ObjectId;
use tokio::sync::RwLock;

use crate::web::{
    accounts::AccountQuery,
    balances::BalanceQuery,
    categories::CategoryQuery,
    expenses::ExpenseQuery,
    users::{UserMutation, UserQuery},
};

pub struct Context {
    pub connection: mongodb::Database,
    pub config: crate::config::Config,
    pub user_id: Arc<RwLock<Option<ObjectId>>>,
}

impl juniper::Context for Context {}
pub struct Query;

pub struct Mutation;

pub type Schema = juniper::RootNode<'static, Query, Mutation, juniper::EmptySubscription<Context>>;

pub async fn require_user(context: &Context) -> juniper::FieldResult<ObjectId> {
    context.user_id.read().await.ok_or(juniper::FieldError::new(
        "User is not logged in",
        juniper::Value::null(),
    ))
}

pub async fn graphql_handler(
    connection: &rocket::State<mongodb::Database>,
    config: &rocket::State<crate::config::Config>,
    cookies: &rocket::http::CookieJar<'_>,
    request: juniper_rocket::GraphQLRequest,
    schema: &rocket::State<Schema>,
) -> juniper_rocket::GraphQLResponse {
    let user_id = cookies
        .get_private("user_id")
        .and_then(|c| c.value().parse().ok());
    let user_id_writer = Arc::new(RwLock::new(user_id));

    let r = request
        .execute(
            schema,
            &Context {
                config: (*config).clone(),
                connection: (*connection).clone(),
                user_id: user_id_writer.clone(),
            },
        )
        .await;

    let new_user_id = user_id_writer.read().await;
    if *new_user_id != user_id {
        if let Some(uid) = *new_user_id {
            info!("Login for user_id {}", uid);
            cookies.add_private(rocket::http::Cookie::new("user_id", uid.to_string()));
        } else {
            info!("Logout for user_id {}", user_id.unwrap());
            cookies.remove_private(rocket::http::Cookie::named("user_id"));
        }
    }

    r
}

#[graphql_object(
    Context = Context,
)]
impl Query {
    fn apiVersion() -> &'static str {
        "1.0"
    }

    fn users() -> UserQuery {
        UserQuery
    }

    fn accounts() -> AccountQuery {
        AccountQuery
    }

    fn balances() -> BalanceQuery {
        BalanceQuery
    }

    fn expenses() -> ExpenseQuery {
        ExpenseQuery
    }

    fn categories() -> CategoryQuery {
        CategoryQuery
    }
}

#[graphql_object(
    Context = Context,
)]
impl Mutation {
    fn users() -> UserMutation {
        UserMutation
    }
}
