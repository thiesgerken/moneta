use moneta::{cli, config::Config, graphql};

use dotenvy::dotenv;
use log::{LevelFilter, debug, info};
use simplelog::{ColorChoice, SimpleLogger, TermLogger, TerminalMode};
use tokio::runtime::Builder;

pub fn initialize_logging(verbosity: i8) {
    let level = match verbosity {
        -1 => LevelFilter::Warn,
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        n => {
            if n > 0 {
                LevelFilter::Trace
            } else {
                LevelFilter::Error
            }
        }
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

fn main() {
    dotenv().ok();

    let runtime = Builder::new_multi_thread()
        .thread_name("moneta-main")
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        let matches = cli::build().get_matches();
        let settings_builder = config::ConfigBuilder::<config::builder::DefaultState>::default()
            .add_source(config::Environment::with_prefix("moneta").separator("__"))
            .set_override_option(
                "verbosity",
                matches
                    .get_one::<u8>("verbosity")
                    .map(|x| *x as i64)
                    .filter(|&x| x != 0),
            )
            .unwrap();

        let settings = settings_builder.build().expect("unable to parse settings");
        let config = settings
            .try_deserialize::<Config>()
            .expect("unable to parse settings into Config struct");

        initialize_logging(config.verbosity);
        debug!("config={:?}", config);

        let connection =
            moneta::connect_mongo(&config.database.url, &config.database.name, "moneta-web")
                .await
                .expect("error connecting and migrating to db");

        if let Some(sub_matches) = matches.subcommand_matches("user") {
            cli::user::handle(connection, sub_matches).await;
        } else if let Some(sub_matches) = matches.subcommand_matches("account") {
            cli::account::handle(connection, sub_matches).await;
        } else if let Some(sub_matches) = matches.subcommand_matches("periodic") {
            cli::periodic::handle(connection, sub_matches).await;
        } else if let Some(sub_matches) = matches.subcommand_matches("import") {
            cli::import::handle(connection, sub_matches).await;
        } else if matches.subcommand_matches("serve").is_some() {
            graphql::handle(config, connection).await;
        } else {
            cli::build()
                .print_long_help()
                .expect("unable to print help");
            println!();
        }
    });
}
