use moneta::cli::Config;
use moneta::initialize_logging;
use moneta::*;

use log::debug;
use tokio::runtime::Builder;

fn main() {
    let runtime = Builder::new_multi_thread()
        .thread_name("moneta-main")
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        let matches = cli::build(true).get_matches();
        initialize_logging(matches.occurrences_of("v") as u32);

        // setup default settings and parse overrides from config files
        let mut settings = cli::initialize_settings(matches.value_of("config"))
            .expect("unable to initialize settings");

        // parse overrides from CLI and modify settings
        cli::merge_settings(&matches, &mut settings).expect("unable to merge flags from CLI");

        let config = settings
            .try_into::<Config>()
            .expect("unable to parse settings into Config struct");

        debug!("config={:?}", &config);

        let connection = connect(&config.database).expect("error connecting and migrating to db");

        if let Some(sub_matches) = matches.subcommand_matches("user") {
            cli::user::handle(&connection, sub_matches);
        } else if let Some(sub_matches) = matches.subcommand_matches("import") {
            cli::import::handle(&connection, sub_matches);
        } else if let Some(sub_matches) = matches.subcommand_matches("export") {
            cli::export::handle(&connection, sub_matches);
        } else if let Some(sub_matches) = matches.subcommand_matches("serve") {
            std::mem::drop(connection); // `serve::handle` creates its own connections
            cli::serve::handle(sub_matches, config.web, config.database, config.verbosity).await;
        } else {
            cli::build(true)
                .print_long_help()
                .expect("unable to print help");
            println!();
        }
    });
}
