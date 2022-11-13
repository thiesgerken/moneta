use std::sync::Arc;

use juniper::graphql_object;
use log::info;
use mongodb::bson::oid::ObjectId;
use tokio::sync::RwLock;

use crate::graphql::{
    accounts::{AccountMutation, AccountQuery},
    analysis::AnalysisQuery,
    balances::{BalanceMutation, BalanceQuery},
    categories::{CategoryMutation, CategoryQuery},
    expenses::{ExpenseMutation, ExpenseQuery},
    users::{UserMutation, UserQuery},
};

use super::{
    accounts::{get_account_loader, AccountLoader},
    categories::{get_category_loader, CategoryLoader},
    users::{get_user_loader, UserLoader},
};

pub struct Context {
    pub connection: mongodb::Database,
    pub config: crate::config::Config,
    pub user_id: Arc<RwLock<Option<ObjectId>>>,

    pub account_loader: AccountLoader,
    pub category_loader: CategoryLoader,
    pub user_loader: UserLoader,
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
                account_loader: get_account_loader((*connection).clone()),
                category_loader: get_category_loader((*connection).clone()),
                user_loader: get_user_loader((*connection).clone()),
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

    fn analysis() -> AnalysisQuery {
        AnalysisQuery
    }
}

#[graphql_object(
    Context = Context,
)]
impl Mutation {
    fn users() -> UserMutation {
        UserMutation
    }

    fn accounts() -> AccountMutation {
        AccountMutation
    }

    fn balances() -> BalanceMutation {
        BalanceMutation
    }

    fn expenses() -> ExpenseMutation {
        ExpenseMutation
    }

    fn categories() -> CategoryMutation {
        CategoryMutation
    }
}
