pub mod cli;
pub mod config;
pub mod graphql;
pub mod import;
pub mod models;
pub mod queries;
pub mod rendering;
pub mod rules;
pub mod schema;
pub mod schema_views;

#[macro_use]
extern crate rocket;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use diesel_async::{
    AsyncConnection, AsyncPgConnection, SimpleAsyncConnection,
    pooled_connection::{AsyncDieselConnectionManager, ManagerConfig, deadpool},
};
use futures::FutureExt;

pub type Pool = deadpool::Pool<AsyncPgConnection>;

pub const MIGRATIONS: diesel_migrations::EmbeddedMigrations =
    diesel_migrations::embed_migrations!("migrations");

/// Brings the schema up to date, on a connection of its own.
///
/// diesel's migration harness is synchronous, so it runs on a blocking thread through the
/// async connection wrapper rather than on the pool.
pub async fn migrate(database_url: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
    use diesel_migrations::MigrationHarness;

    let conn = AsyncPgConnection::establish(database_url).await?;
    let mut wrapper = AsyncConnectionWrapper::<AsyncPgConnection>::from(conn);

    tokio::task::spawn_blocking(move || {
        for name in wrapper.run_pending_migrations(MIGRATIONS)? {
            info!("applied migration {name}");
        }
        Ok(())
    })
    .await?
}

pub fn pool(database_url: &str, app_name: &str) -> Result<Pool, Box<dyn std::error::Error>> {
    let app_name = app_name.to_owned();

    let mut config = ManagerConfig::default();
    config.custom_setup = Box::new(move |url| {
        let app_name = app_name.clone();
        async move {
            let mut conn = AsyncPgConnection::establish(url).await?;

            // The rendered views embed timestamps in jsonb, where the text carries the
            // session's UTC offset. chrono reads any offset, so this is about keeping the
            // representation identical wherever the backend runs.
            conn.batch_execute(&format!(
                "SET TIME ZONE 'UTC'; SET application_name = '{}'",
                app_name.replace('\'', "")
            ))
            .await
            .map_err(diesel::ConnectionError::CouldntSetupConfiguration)?;

            Ok(conn)
        }
        .boxed()
    });

    let manager =
        AsyncDieselConnectionManager::<AsyncPgConnection>::new_with_config(database_url, config);
    Ok(deadpool::Pool::builder(manager).build()?)
}

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
