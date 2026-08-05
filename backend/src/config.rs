use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct Config {
    pub verbosity: i8,
    pub database: DatabaseConfig,
    pub api: ApiConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct DatabaseConfig {
    pub url: String,

    /// source for `moneta migrate-mongo`, unused otherwise
    pub mongo_url: String,
    /// database name within `mongo_url`
    pub name: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://moneta:moneta@localhost:5433/moneta".into(),
            mongo_url: "mongodb://localhost:27018".into(),
            name: "moneta".into(),
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
