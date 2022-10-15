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

pub struct Config {
    pub port: u16,
    pub address: String,
    pub secret_key: Option<String>,
    pub log_level: LogLevel,
}

pub async fn handle(config: Config, connection: mongodb::Database) {
    let limits = Limits::new()
        .limit("forms", 64_i32.kibibytes())
        .limit("json", 10_i32.mebibytes());

    let mut rocket_config = rocket::config::Config::figment()
        .merge(("port", config.port))
        .merge(("address", config.address.clone()))
        .merge(("limits", limits))
        .merge(("log_level", config.log_level));

    if let Some(key) = config.secret_key.clone() {
        rocket_config = rocket_config.merge(("secret_key", key));
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
