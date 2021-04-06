pub mod cli;
pub mod enums;
pub mod models;
pub mod queries;
pub mod rendering;
pub mod schema;
pub mod serialization;
pub mod web;

#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

#[macro_use]
extern crate rocket_contrib;

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate rust_embed;

use argonautica::{Hasher, Verifier};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use log::info;
use simplelog::{LevelFilter, SimpleLogger, TermLogger, TerminalMode};
use std::error::Error;

embed_migrations!();

pub fn connect(database_url: &str) -> Result<PgConnection, Box<dyn Error>> {
    let conn = PgConnection::establish(&database_url)?;
    embedded_migrations::run_with_output(&conn, &mut std::io::stdout())?;
    Ok(conn)
}

pub fn verify_password(pw: &str, hash: &str) -> bool {
    Verifier::default()
        .with_hash(hash)
        .with_password(pw)
        .verify()
        .unwrap_or(false)
}

pub fn hash_password(pw: &str) -> String {
    Hasher::default()
        .opt_out_of_secret_key(true)
        .configure_iterations(32)
        .with_password(pw)
        .hash()
        .unwrap()
}

pub fn initialize_logging(verbosity: u32) {
    let level = match verbosity {
        // 0 => LevelFilter::Warn,
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    let mut log_config = simplelog::ConfigBuilder::new();
    log_config.set_time_format_str("");

    // log_config.add_filter_ignore_str("html5ever");
    // log_config.add_filter_ignore_str("hyper");
    // log_config.add_filter_ignore_str("reqwest");
    // log_config.add_filter_ignore_str("selectors");

    // log_config.add_filter_allow_str("moneta");

    if let Err(e) = TermLogger::init(level, log_config.build(), TerminalMode::Mixed) {
        SimpleLogger::init(level, log_config.build())
            .expect("TermLogger and SimpleLogger failed to init!?");
        info!(
            "Falling back to SimpleLogger because TermLogger init failed: {}",
            e
        );
    }
}
