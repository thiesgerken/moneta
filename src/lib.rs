pub mod cli;
pub mod import;
pub mod models;
pub mod queries;
pub mod web;
pub mod rendering;

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate rust_embed;

use argonautica::{Hasher, Verifier};
use simplelog::{ColorChoice, LevelFilter, SimpleLogger, TermLogger, TerminalMode};

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

    // log_config.add_filter_ignore_str("html5ever");
    // log_config.add_filter_ignore_str("hyper");
    // log_config.add_filter_ignore_str("reqwest");
    // log_config.add_filter_ignore_str("selectors");
    // log_config.add_filter_allow_str("moneta");

    if let Err(e) = TermLogger::init(
        level,
        log_config.build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    ) {
        SimpleLogger::init(level, log_config.build())
            .expect("TermLogger and SimpleLogger failed to init!?");
        info!(
            "Falling back to SimpleLogger because TermLogger init failed: {}",
            e
        );
    }
}

pub async fn connect(
    database_url: &str,
    database_name: &str,
    app_name: &str,
) -> Result<mongodb::Database, mongodb::error::Error> {
    let mut client_options = mongodb::options::ClientOptions::parse(database_url).await?;
    client_options.app_name = Some(app_name.to_owned());

    let client = mongodb::Client::with_options(client_options)?;
    Ok(client.database(database_name))
}
