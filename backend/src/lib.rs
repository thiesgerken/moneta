pub mod cli;
pub mod config;
pub mod graphql;
pub mod import;
pub mod models;
pub mod queries;
pub mod rendering;

#[macro_use]
extern crate rocket;

use argonautica::{Hasher, Verifier};
use models::rendering::{RenderedBalance, RenderedExpense};
use mongodb::bson::doc;

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
    let db = client.database(database_name);

    db.collection::<RenderedExpense>("expenses_rendered")
        .create_indexes(
            [
                mongodb::IndexModel::builder()
                    .keys(doc! {"upstreamId": 1})
                    .build(),
                mongodb::IndexModel::builder()
                    .keys(doc! {"userId": 1})
                    .build(),
                mongodb::IndexModel::builder()
                    .keys(doc! {"userId": 1, "bookingStart": 1})
                    .build(),
                mongodb::IndexModel::builder()
                    .keys(doc! {"userId": 1, "bookingEnd": 1})
                    .build(),
                mongodb::IndexModel::builder()
                    .keys(doc! {"userId": 1, "isTemplate": 1, "isUnchecked": 1, "isIncomplete": 1, "bookingStart": -1})
                    .build(),
                mongodb::IndexModel::builder()
                    .keys(doc! {"userId": 1, "isTemplate": 1, "isUnchecked": 1, "isIncomplete": 1, "bookingEnd": -1})
                    .build(),
                mongodb::IndexModel::builder()
                    .keys(doc! {"userId": 1, "isTemplate": 1, "isUnchecked": -1, "isIncomplete": -1, "bookingStart": -1})
                    .build(),
                mongodb::IndexModel::builder()
                    .keys(doc! {"userId": 1, "isTemplate": 1, "isUnchecked": -1, "isIncomplete": -1, "bookingEnd": -1})
                    .build(),
                mongodb::IndexModel::builder()
                    .keys(doc! {"userId": 1, "transactions.date": 1})
                    .build(),
                mongodb::IndexModel::builder()
                    .keys(doc! {"userId": 1, "transactions.accountId": 1, "transactions.date": 1})
                    .build(),
            ]
            ,
        )
        .await?;

    db.collection::<RenderedBalance>("balances_rendered")
        .create_indexes(
            [
                mongodb::IndexModel::builder()
                    .keys(doc! {"upstreamId": 1})
                    .build(),
                mongodb::IndexModel::builder()
                    .keys(doc! {"userId": 1})
                    .build(),
                mongodb::IndexModel::builder()
                    .keys(doc! {"userId": 1, "date": 1})
                    .build(),
                mongodb::IndexModel::builder()
                    .keys(doc! {"userId": 1, "accountId": 1, "date": 1})
                    .build(),
            ]
            ,
        )
        .await?;

    Ok(db)
}
