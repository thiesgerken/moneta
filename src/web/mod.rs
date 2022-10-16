pub mod accounts;
pub mod authentication;
pub mod balances;
pub mod categories;
pub mod expenses;
pub mod user;
mod util;

use std::sync::Arc;

use futures::{StreamExt, TryStreamExt};
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use rocket::config::LogLevel;
use rocket::data::{Limits, ToByteUnit};
use rocket::http::CookieJar;
use rocket_okapi::{openapi_get_routes, swagger_ui::make_swagger_ui, swagger_ui::SwaggerUIConfig};
use tokio::sync::Mutex;

use crate::config::Config;
use crate::models::{Account, ObjectIdWrapper, User};
use crate::queries;
use crate::web::user::UserInfo;

use juniper::{graphql_object, EmptySubscription, FieldError, FieldResult, Value};
use rocket::{response::content, State};

#[rocket::get("/")]
fn graphiql() -> content::RawHtml<String> {
    juniper_rocket::graphiql_source("/graphql", None)
}

#[rocket::get("/graphql?<request>")]
async fn get_graphql_handler(
    connection: &State<mongodb::Database>,
    config: &State<crate::config::Config>,
    cookies: &CookieJar<'_>,
    request: juniper_rocket::GraphQLRequest,
    schema: &State<Schema>,
) -> juniper_rocket::GraphQLResponse {
    // sadly, we can't do login via gql because of lifetime issues.
    // however, at least that means that we can do one auth check for all requests here
    let user_id = cookies
        .get_private("user_id")
        .and_then(|c| c.value().parse().ok());
    if let Some(user_id) = user_id {
        request
            .execute(
                &*schema,
                &Context {
                    config: (*config).clone(),
                    connection: (*connection).clone(),
                    user_id,
                    user_test: Mutex::new(Arc::new(user_id)),
                },
            )
            .await
    } else {
        juniper_rocket::GraphQLResponse::error(FieldError::new(
            "User is not logged in",
            Value::null(),
        ))
    }
}

#[rocket::post("/graphql", data = "<request>")]
async fn post_graphql_handler(
    connection: &State<mongodb::Database>,
    config: &State<crate::config::Config>,
    cookies: &CookieJar<'_>,
    request: juniper_rocket::GraphQLRequest,
    schema: &State<Schema>,
) -> juniper_rocket::GraphQLResponse {
    // sadly, we can't do login via gql because of lifetime issues.
    // however, at least that means that we can do one auth check for all requests here
    let user_id = cookies
        .get_private("user_id")
        .and_then(|c| c.value().parse().ok());
    if let Some(user_id) = user_id {
        let mutex = Arc::new(user_id);

        let r = request
            .execute(
                &*schema,
                &Context {
                    config: (*config).clone(),
                    connection: (*connection).clone(),
                    user_id,
                    user_test: mutex.clone(),
                },
            )
            .await;

        let x: ObjectId = mutex.;

        r
    } else {
        juniper_rocket::GraphQLResponse::error(FieldError::new(
            "User is not logged in",
            Value::null(),
        ))
    }
}

pub struct Context {
    pub connection: mongodb::Database,
    pub config: crate::config::Config,
    pub user_id: ObjectId,
    pub user_test: Arc<ObjectId>,
}

impl juniper::Context for Context {}
struct Query;

#[graphql_object(
    Context = Context,
)]
impl Query {
    fn apiVersion() -> &'static str {
        "1.0"
    }

    async fn user(context: &Context) -> FieldResult<UserInfo> {
        let user =
            queries::query_element_by_id(&context.connection, "users", context.user_id).await?;

        if let Some(user) = user {
            Ok(UserInfo::new(user))
        } else {
            Err(FieldError::new(
                "User does not exist anymore",
                Value::null(),
            ))
        }
    }

    async fn users(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
    ) -> FieldResult<Vec<UserInfo>> {
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

    pub async fn accounts(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
        filter: Option<String>,
        sort: Option<String>,
    ) -> FieldResult<Vec<Account>> {
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

    pub async fn account(context: &Context, id: ObjectIdWrapper) -> FieldResult<Account> {
        let collection = context.connection.collection::<Account>("accounts");
        let entry = collection.find_one(doc! {"_id":*id}, None).await?;

        if let Some(entry) = entry {
            Ok(entry)
        } else {
            Err(FieldError::new("Account does not exist", Value::null()))
        }
    }
}

struct Mutation;

#[graphql_object(
    Context = Context,
)]
impl Mutation {
    fn double(context: &Context, number: i32) -> FieldResult<i32> {
        Ok(2 * number)
    }
}

type Schema = juniper::RootNode<'static, Query, Mutation, EmptySubscription<Context>>;

pub async fn handle(config: Config, connection: mongodb::Database) {
    let level = match config.verbosity {
        -1 => LogLevel::Critical,
        0 => LogLevel::Normal,
        n => {
            if n > 0 {
                LogLevel::Debug
            } else {
                LogLevel::Off
            }
        }
    };

    let limits = Limits::new()
        .limit("forms", 64_i32.kibibytes())
        .limit("json", 10_i32.mebibytes());

    let mut rocket_config = rocket::config::Config::figment()
        .merge(("port", config.api.port))
        .merge(("address", config.api.address.clone()))
        .merge(("limits", limits))
        .merge(("log_level", level));

    if !config.api.secret_key.is_empty() {
        rocket_config = rocket_config.merge(("secret_key", config.api.secret_key.clone()));
    }

    let _rocket = rocket::custom(rocket_config)
        .manage(config.clone())
        .manage(connection.clone())
        .manage(Schema::new(
            Query,
            Mutation,
            EmptySubscription::<Context>::new(),
        ))
        .mount(
            "/",
            rocket::routes![graphiql, get_graphql_handler, post_graphql_handler],
        )
        .mount(
            "/api/",
            openapi_get_routes![
                user::login,
                user::logout,
                accounts::delete,
                accounts::update,
                accounts::create,
                categories::list,
                categories::get,
                categories::delete,
                categories::update,
                categories::create,
                balances::list,
                balances::get,
                balances::delete,
                balances::update,
                balances::create,
                expenses::list,
                expenses::get,
                expenses::delete,
                expenses::update,
                expenses::create,
            ],
        )
        .mount(
            "/api/doc/",
            make_swagger_ui(&SwaggerUIConfig {
                url: "../openapi.json".to_owned(),
                ..Default::default()
            }),
        )
        .launch()
        .await
        .expect("server died");
}
