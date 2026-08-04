pub mod cli;
pub mod config;
pub mod graphql;
pub mod import;
pub mod models;
pub mod queries;
pub mod rendering;

#[macro_use]
extern crate rocket;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use models::rendering::{RenderedBalance, RenderedExpense};
use mongodb::bson::doc;

/// Parameters are taken from the stored PHC string, so hashes written by older
/// versions (argonautica, m=4096/t=32/p=num_cpus) keep verifying.
pub fn verify_password(pw: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|parsed| {
            Argon2::default()
                .verify_password(pw.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

pub fn hash_password(pw: &str) -> String {
    Argon2::default()
        .hash_password(pw.as_bytes(), &SaltString::generate(&mut OsRng))
        .expect("unable to hash password")
        .to_string()
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
        .create_indexes([
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
        ])
        .await?;

    Ok(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_roundtrip() {
        let h = hash_password("correct horse battery staple");
        assert!(verify_password("correct horse battery staple", &h));
        assert!(!verify_password("wrong horse battery staple", &h));
        assert!(!verify_password(
            "correct horse battery staple",
            "not a phc string"
        ));
    }

    /// hashes written by argonautica used different parameters; they must keep verifying
    #[test]
    fn verifies_legacy_parameters() {
        let legacy = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(4096, 32, 8, Some(32)).unwrap(),
        );
        let h = legacy
            .hash_password(b"hunter2", &SaltString::generate(&mut OsRng))
            .unwrap()
            .to_string();

        assert!(verify_password("hunter2", &h));
        assert!(!verify_password("hunter3", &h));
    }
}
