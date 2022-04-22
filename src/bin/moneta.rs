use moneta::cli;

use log::debug;
use tokio::runtime::Builder;

fn main() {
    let runtime = Builder::new_multi_thread()
        .thread_name("moneta-main")
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        let matches = cli::build().get_matches();
        moneta::initialize_logging(matches.occurrences_of("v") as u32);

        // setup default settings and parse overrides from config files
        let mut settings_builder = cli::initialize_settings(matches.value_of("config"));

        // parse overrides from CLI and modify settings
        settings_builder =
            cli::merge_settings(&matches, settings_builder).expect("unable to parse cli overrides");

        let settings = settings_builder.build().expect("unable to parse settings");
        let config = settings
            .try_deserialize::<cli::Config>()
            .expect("unable to parse settings into Config struct");

        debug!("config={:?}", &config);

        let connection = moneta::connect(&config.database_url, &config.database_name, "moneta-web")
            .await
            .expect("error connecting and migrating to db");

        if let Some(sub_matches) = matches.subcommand_matches("user") {
            // cli::user::handle(&connection, sub_matches);
        } else if let Some(sub_matches) = matches.subcommand_matches("import") {
            // cli::import::handle(&connection, sub_matches);
        } else if let Some(sub_matches) = matches.subcommand_matches("export") {
            // cli::export::handle(&connection, sub_matches);
        } else if let Some(sub_matches) = matches.subcommand_matches("serve") {
            cli::serve::handle(connection, sub_matches, config.web, config.verbosity).await;
        } else {
            cli::build()
                .print_long_help()
                .expect("unable to print help");
            println!();
        }
    });
}
