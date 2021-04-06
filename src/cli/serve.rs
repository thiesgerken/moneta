use crate::web;

use clap::ArgMatches;
use clap::{App, Arg, SubCommand};
use rocket::config::LogLevel;
use serde::{Deserialize, Serialize};

pub fn build() -> App<'static, 'static> {
    SubCommand::with_name("serve")
    .about("serve web interface")
    .arg(
      Arg::with_name("port")
        .long("port")
        .short("p")
        .value_name("port")
        .help("customize listening port [default: 'postgres://localhost/stocks']"),
    )
    .arg(
      Arg::with_name("address")
        .long("address")
        .value_name("address")
        .help("customize listening address [default: 'localhost']"),
    )
    .arg(
      Arg::with_name("secret")
        .long("secret")
        .value_name("key")
        .help("secret key for cookie encryption [default: randomly generated]; use `openssl rand -base64 32` to create one"),
    )
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Config {
    pub secret_key: String,
    pub port: i64,
    pub address: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            secret_key: String::new(),
            port: 8484,
            address: "127.0.0.1".into(),
        }
    }
}

pub async fn handle(
    sub_matches: &ArgMatches<'_>,
    mut config: Config,
    database: String,
    verbosity: i64,
) {
    if let Some(y) = sub_matches.value_of("port") {
        config.port = y.parse().expect("cannot parse port");
    }

    if let Some(y) = sub_matches.value_of("secret") {
        config.secret_key = y.into();
    }

    if let Some(y) = sub_matches.value_of("address") {
        config.address = y.into();
    }

    let level = match verbosity {
        0 => LogLevel::Critical,
        1 => LogLevel::Normal,
        _ => LogLevel::Debug,
    };

    web::handle(web::Config {
        port: config.port as u16,
        address: config.address,
        secret_key: if config.secret_key.is_empty() {
            None
        } else {
            Some(config.secret_key)
        },
        log_level: level,
        database_url: database,
    })
    .await;
}
