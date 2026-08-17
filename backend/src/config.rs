use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct Config {
    pub verbosity: i8,
    pub database: DatabaseConfig,
    pub api: ApiConfig,
    pub stockdb: StockdbConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct DatabaseConfig {
    pub url: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://moneta:moneta@localhost:5433/moneta".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ApiConfig {
    pub secret_key: String,
    pub port: u16,
    pub address: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            secret_key: String::new(),
            port: 8181,
            address: "127.0.0.1".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct StockdbConfig {
    /// Where stockdb answers, without a trailing slash, e.g. `https://stocks.gerken.haus`.
    /// Empty switches the whole integration off: `investments` then reports nothing, which is
    /// what every deployment that has no stockdb wants.
    pub url: String,

    /// Where a *browser* reaches stockdb, for the link on a linked depot. In the cluster `url`
    /// is a Service that resolves nowhere outside it, which would make that link dead; where
    /// the two are the same, leaving this empty falls back to `url`.
    pub public_url: String,

    /// How long to wait for it. This sits on a page load, so a stockdb that hangs has to
    /// become a missing number quickly rather than a spinner that never resolves.
    pub timeout_seconds: u64,
}

impl Default for StockdbConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            public_url: String::new(),
            timeout_seconds: 5,
        }
    }
}
