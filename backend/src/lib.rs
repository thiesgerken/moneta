pub mod accrual;
pub mod api_keys;
pub mod cli;
pub mod config;
pub mod graphql;
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
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use diesel_async::{
    AsyncConnection, AsyncPgConnection, SimpleAsyncConnection,
    pooled_connection::{AsyncDieselConnectionManager, ManagerConfig, deadpool},
};
use futures::FutureExt;

pub type Pool = deadpool::Pool<AsyncPgConnection>;

pub const MIGRATIONS: diesel_migrations::EmbeddedMigrations =
    diesel_migrations::embed_migrations!("migrations");

/// One a second, so a database that is genuinely gone still fails inside half a minute rather
/// than leaving a pod that looks alive and answers nothing.
const DATABASE_CONNECT_ATTEMPTS: u32 = 30;

/// deadpool's default is derived from the core count, which in a container with a 1 CPU limit
/// left a pool small enough that a handful of concurrent dashboard requests emptied it.
const POOL_SIZE: usize = 16;

/// Brings the schema up to date, on a connection of its own.
///
/// diesel's migration harness is synchronous, so it runs on a blocking thread through the
/// async connection wrapper rather than on the pool.
pub async fn migrate(database_url: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
    use diesel_migrations::MigrationHarness;

    // Postgres is not always up when we are. A rolling restart schedules the new pod while the
    // old one still holds its connections, and a cold cluster starts both at once; the first
    // connect then fails with "error connecting to server". Panicking there costs a container
    // restart, and without a readiness probe the pod takes traffic for the whole of it -- which
    // is a request that never answers rather than one that fails.
    let mut attempt = 0u32;
    let conn = loop {
        match AsyncPgConnection::establish(database_url).await {
            Ok(conn) => break conn,
            Err(e) => {
                attempt += 1;
                if attempt >= DATABASE_CONNECT_ATTEMPTS {
                    return Err(e.into());
                }

                warn!("database not reachable yet ({e}), retrying in a second");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    };

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

    // A wait that cannot time out is how one bug takes the whole backend down: a resolver that
    // holds a connection while asking for a second one empties the pool under concurrency, and
    // every request after that waits forever. Rocket keeps accepting them, nothing is logged
    // past "Matched", and only a restart clears it. Failing the request says what happened.
    Ok(deadpool::Pool::builder(manager)
        .max_size(POOL_SIZE)
        .runtime(::deadpool::Runtime::Tokio1)
        .wait_timeout(Some(std::time::Duration::from_secs(15)))
        .build()?)
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
        .hash_password(pw.as_bytes())
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
        let h = legacy.hash_password(b"hunter2").unwrap().to_string();

        assert!(verify_password("hunter2", &h));
        assert!(!verify_password("hunter3", &h));
    }
}
