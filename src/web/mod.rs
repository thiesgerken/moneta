// pub mod accounts;
// pub mod balances;
// pub mod categories;
// pub mod expenses;
pub mod static_files;
// pub mod user;
mod util;

use rocket::config::LogLevel;
use rocket::data::{Limits, ToByteUnit};

pub struct Config {
    pub port: u16,
    pub address: String,
    pub secret_key: Option<String>,
    pub log_level: LogLevel,
}

pub async fn handle(config: Config, connection: mongodb::Database) {
    let limits = Limits::new()
        .limit("forms", 64.kibibytes())
        .limit("json", 10.mebibytes());

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
            routes![
                // user::login,
                // user::logout,
                // user::info,
                // user::list,
                // accounts::list,
                // accounts::get,
                // accounts::delete,
                // accounts::update,
                // accounts::create,
                // balances::list,
                // balances::get,
                // balances::query,
                // balances::info,
                // balances::delete,
                // balances::update,
                // balances::create,
                // categories::list,
                // categories::get,
                // categories::delete,
                // categories::update,
                // categories::create,
                // expenses::list,
                // expenses::get,
                // expenses::query,
                // expenses::info,
                // categories::delete,
                // categories::update,
                // categories::create,
            ],
        )
        .mount("/", routes![static_files::serve, static_files::index])
        .launch()
        .await
        .expect("server died");
}
