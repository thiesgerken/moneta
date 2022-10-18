pub mod cli;
pub mod config;
pub mod import;
pub mod models;
pub mod queries;
pub mod rendering;
pub mod web;

#[macro_use]
extern crate rocket;

use argonautica::{Hasher, Verifier};

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
