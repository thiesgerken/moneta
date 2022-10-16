pub mod accounts;
pub mod authentication;
pub mod balances;
pub mod categories;
pub mod expenses;
pub mod user;
mod util;

use rocket::config::LogLevel;
use rocket::data::{Limits, ToByteUnit};
use rocket_okapi::{openapi_get_routes, swagger_ui::make_swagger_ui, swagger_ui::SwaggerUIConfig};

use crate::config::Config;

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

    rocket::custom(rocket_config)
        .manage(config)
        .manage(connection)
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
