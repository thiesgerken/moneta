pub mod account;
pub mod import;
pub mod serve;
pub mod user;

use clap::{crate_authors, crate_name, crate_version, Arg, Command};
use config::ConfigError;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub fn build() -> Command<'static> {
    Command::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about("Personal expense management and analysis")
        .arg(Arg::new("config")
                .short('c')
                .long("config")
                .value_name("file")
                .help("read configuration from this file (instead of config.* in '/etc/moneta' and moneta.* in '.')"))
        .arg(Arg::new("v")
                .short('v')
                .long("verbose")
                .multiple_occurrences(true)
                .help("specify multiple times (up to three) to increase verbosity level"))
                .arg(Arg::new("database_url")
                .long("database_url")
                .help("database url [default: 'localhost:27017']")
                .value_name("url"))
        .arg(Arg::new("database_name")
                .long("database_name")
                .help("database name [default: 'moneta']")
                .value_name("name"))
        .subcommand(user::build())
        .subcommand(import::build())
        .subcommand(account::build())
        .subcommand(serve::build())
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Config {
    pub verbosity: i64,
    pub database_url: String,
    pub database_name: String,
    pub web: serve::Config,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_url: "mongodb://localhost:27017".into(),
            database_name: "moneta".into(),
            verbosity: 0,
            web: Default::default(),
        }
    }
}

// set default settings and load overrides from config files
// files that are considered: base names /etc/moneta/config and ./moneta
// fails, if additional_file does not exist or cannot be read
pub fn initialize_settings(
    additional_file: Option<&str>,
) -> config::ConfigBuilder<config::builder::DefaultState> {
    let settings_builder = config::Config::builder();

    let settings_builder = if let Some(p) = additional_file {
        settings_builder.add_source(config::File::from(Path::new(p)).required(true))
    } else {
        settings_builder
            .add_source(config::File::with_name("/etc/moneta/config").required(false))
            .add_source(config::File::with_name("./moneta").required(false))
    };

    settings_builder.add_source(config::Environment::with_prefix("moneta").separator("__"))
}

// parse the command line flags and add them as overrides to settings
pub fn merge_settings(
    opts: &clap::ArgMatches,
    mut settings: config::ConfigBuilder<config::builder::DefaultState>,
) -> Result<config::ConfigBuilder<config::builder::DefaultState>, ConfigError> {
    let ov = vec!["database_url", "database_name"];

    for x in ov.iter() {
        settings = settings.set_override_option(x, opts.value_of(x))?;
    }

    settings.set_override("verbosity", opts.occurrences_of("v") as i64)
}
