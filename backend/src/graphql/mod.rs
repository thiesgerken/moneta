pub mod accounts;
pub mod analysis;
pub mod api_keys;
pub mod balances;
pub mod cache;
pub mod categories;
pub mod expenses;
pub mod investments;
pub(crate) mod root;
pub mod rules;
pub mod scoped;
pub mod users;
mod util;

use rocket_cors::{AllowedOrigins, CorsOptions};

use rocket::State;
use rocket::config::LogLevel;
use rocket::data::{Limits, ToByteUnit};
use rocket::http::{CookieJar, Method};

use crate::config::Config;
use cache::AnalysisCache;

#[rocket::get("/")]
fn graphiql() -> rocket::response::content::RawHtml<String> {
    juniper_rocket::graphiql_source("/graphql", None)
}

/// The schema as SDL, for `frontend/schema.graphql`.
///
/// Building the root node touches no database, so this works without one. `houdini
/// pull-schema` needs a running server and therefore a database behind it, which makes
/// regenerating the schema unnecessarily hard when all that changed is a resolver signature.
pub fn sdl() -> String {
    root::Schema::new(
        root::Query,
        root::Mutation,
        juniper::EmptySubscription::<root::Context>::new(),
    )
    .as_sdl()
}

/// Queries only. A GET is what a link, an image or a top-level navigation from another site
/// sends, with the cookies (and behind a login proxy, the proxy's) attached, so a mutation in
/// one would be a write anybody can trigger by getting a reader to open a URL.
#[rocket::get("/graphql?<request..>")]
async fn get_graphql_handler(
    pool: &State<crate::Pool>,
    config: &State<crate::config::Config>,
    cache: &State<std::sync::Arc<AnalysisCache>>,
    cookies: &CookieJar<'_>,
    presented: root::Presented<'_>,
    request: juniper_rocket::GraphQLRequest,
    schema: &State<root::Schema>,
) -> juniper_rocket::GraphQLResponse {
    if root::contains_mutation(&request, schema) {
        return juniper_rocket::GraphQLResponse::error(juniper::FieldError::from(
            "Mutations are only accepted over POST",
        ));
    }
    root::graphql_handler(pool, config, cache, cookies, presented, request, schema).await
}

/// JSON only. An HTML form or a CORS-simple request can send form data or text/plain without a
/// preflight, cookies included; neither can send application/json.
#[rocket::post("/graphql", format = "json", data = "<request>")]
async fn post_graphql_handler(
    pool: &State<crate::Pool>,
    config: &State<crate::config::Config>,
    cache: &State<std::sync::Arc<AnalysisCache>>,
    cookies: &CookieJar<'_>,
    presented: root::Presented<'_>,
    request: juniper_rocket::GraphQLRequest,
    schema: &State<root::Schema>,
) -> juniper_rocket::GraphQLResponse {
    root::graphql_handler(pool, config, cache, cookies, presented, request, schema).await
}

/// Builds the same HTTP application used in production and handler integration tests.
pub fn rocket(config: Config, pool: crate::Pool) -> rocket::Rocket<rocket::Build> {
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

    let cors = CorsOptions::default()
        .allowed_origins(AllowedOrigins::all())
        .allowed_methods(
            vec![Method::Get, Method::Post, Method::Patch]
                .into_iter()
                .map(From::from)
                .collect(),
        )
        .allow_credentials(true);

    rocket::custom(rocket_config)
        .manage(config.clone())
        .manage(pool.clone())
        .manage(std::sync::Arc::new(AnalysisCache::default()))
        .manage(root::Schema::new(
            root::Query,
            root::Mutation,
            juniper::EmptySubscription::<root::Context>::new(),
        ))
        .mount(
            "/",
            rocket::routes![graphiql, get_graphql_handler, post_graphql_handler],
        )
        .attach(cors.to_cors().unwrap())
}

pub async fn handle(config: Config, pool: crate::Pool) {
    if let Err(e) = config.api.validate() {
        panic!("invalid configuration: {e}");
    }
    rocket(config, pool).launch().await.expect("server died");
}
