pub mod accounts;
pub mod authentication;
pub mod balances;
pub mod categories;
pub mod expenses;
pub mod user;
mod util;

use rocket::config::LogLevel;
use rocket::data::{Limits, ToByteUnit};
use rocket::http::CookieJar;
use rocket_okapi::{openapi_get_routes, swagger_ui::make_swagger_ui, swagger_ui::SwaggerUIConfig};
use tokio::sync::Mutex;

use crate::config::Config;

use juniper::{graphql_object, EmptySubscription, FieldResult};
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
    let user_id = cookies
        .get_private("user_id")
        .and_then(|c| c.value().parse().ok());

    request.execute(
        &*schema,
        &Context {
            config: (*config).clone(),
            connection: (*connection).clone(),
            user_id,
        },
    ).await
}

#[rocket::post("/graphql", data = "<request>")]
async fn post_graphql_handler(
    connection: &State<mongodb::Database>,
    config: &State<crate::config::Config>,
    cookies: &CookieJar<'_>,
    request: juniper_rocket::GraphQLRequest,
    schema: &State<Schema>,
) -> juniper_rocket::GraphQLResponse {
    let user_id = cookies
        .get_private("user_id")
        .and_then(|c| c.value().parse().ok());

    request.execute(
        &*schema,
        &Context {
            config: (*config).clone(),
            connection: (*connection).clone(),
            user_id,
        },
    ).await
}

pub struct Context {
    pub connection: mongodb::Database,
    pub config: crate::config::Config,
    pub user_id: Option<i32>,
    // pub cookies: Mutex< CookieJar<'_>>,
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

    fn countLetters(context: &Context, id: String) -> FieldResult<i32> {
        Ok(id.len() as i32)
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
                user::info,
                user::list,
                accounts::list,
                accounts::get,
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
