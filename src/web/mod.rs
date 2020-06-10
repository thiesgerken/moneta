pub mod accounts;
pub mod balances;
pub mod static_files;
pub mod user;

use rocket::config::{Environment, Limits, LoggingLevel, Value};
use rocket_contrib::databases::diesel;
use std::collections::HashMap;

pub struct Config {
    pub port: u16,
    pub address: String,
    pub secret_key: Option<String>,
    pub log_level: LoggingLevel,
    pub database_url: String,
}

pub fn handle(config: Config) {
    let mut database_config = HashMap::new();
    let mut databases = HashMap::new();

    database_config.insert("url", Value::from(config.database_url.clone()));
    databases.insert("moneta", Value::from(database_config));

    let limits = Limits::new()
        .limit("forms", 64 * 1024)
        .limit("json", 10 * 1024 * 1024);

    let mut rocket_config = rocket::config::Config::build(Environment::Production)
        .extra("databases", databases)
        .port(config.port)
        .address(config.address.clone())
        .limits(limits)
        .log_level(config.log_level);

    if let Some(key) = config.secret_key.clone() {
        rocket_config = rocket_config.secret_key(key);
    }

    let rocket_config = rocket_config
        .finalize()
        .expect("Cannot create web server config");

    rocket::custom(rocket_config)
        .attach(DbConn::fairing())
        .manage(config)
        .mount(
            "/api/",
            routes![
                user::login,
                user::logout,
                user::info,
                accounts::list,
                accounts::get,
                // accounts::delete,
                // accounts::update,
                // accounts::create
                balances::list,
                balances::get,
                // balances::delete,
                // balances::update,
                // balances::create
            ],
        )
        .mount("/", routes![static_files::serve, static_files::index])
        .launch();
}

#[database("moneta")]
pub struct DbConn(diesel::PgConnection);
