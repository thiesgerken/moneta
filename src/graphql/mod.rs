pub mod accounts;
pub mod balances;
pub mod categories;
pub mod expenses;
mod root;
pub mod users;
mod util;

use mongodb::bson::doc;
use rocket::config::LogLevel;
use rocket::data::{Limits, ToByteUnit};
use rocket::http::CookieJar;
use rocket::State;

use crate::config::Config;

#[rocket::get("/")]
fn graphiql() -> rocket::response::content::RawHtml<String> {
    juniper_rocket::graphiql_source("/graphql", None)
}

#[rocket::get("/graphql?<request>")]
async fn get_graphql_handler(
    connection: &State<mongodb::Database>,
    config: &State<crate::config::Config>,
    cookies: &CookieJar<'_>,
    request: juniper_rocket::GraphQLRequest,
    schema: &State<root::Schema>,
) -> juniper_rocket::GraphQLResponse {
    root::graphql_handler(connection, config, cookies, request, schema).await
}

#[rocket::post("/graphql", data = "<request>")]
async fn post_graphql_handler(
    connection: &State<mongodb::Database>,
    config: &State<crate::config::Config>,
    cookies: &CookieJar<'_>,
    request: juniper_rocket::GraphQLRequest,
    schema: &State<root::Schema>,
) -> juniper_rocket::GraphQLResponse {
    root::graphql_handler(connection, config, cookies, request, schema).await
}

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
        .manage(root::Schema::new(
            root::Query,
            root::Mutation,
            juniper::EmptySubscription::<root::Context>::new(),
        ))
        .mount(
            "/",
            rocket::routes![graphiql, get_graphql_handler, post_graphql_handler],
        )
        .launch()
        .await
        .expect("server died");
}
