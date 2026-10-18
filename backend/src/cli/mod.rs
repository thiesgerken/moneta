pub mod account;
pub mod periodic;
pub mod user;

use clap::{Arg, ArgAction, Command, crate_authors, crate_name, crate_version};

pub fn build() -> Command {
    Command::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about("Personal expense management and analysis")
        .arg(
            Arg::new("verbosity")
                .short('v')
                .long("verbosity")
                .action(ArgAction::Count)
                .help("specify multiple times (up to three) to increase verbosity level"),
        )
        .subcommand(user::build())
        .subcommand(account::build())
        .subcommand(periodic::build())
        .subcommand(Command::new("serve").about("serve web interface"))
        .subcommand(
            Command::new("schema")
                .about("print the GraphQL schema as SDL, for frontend/schema.graphql"),
        )
}

#[cfg(test)]
mod tests {
    #[test]
    fn import_is_not_a_supported_command() {
        let error = super::build()
            .try_get_matches_from(["moneta", "import", "--clear"])
            .unwrap_err();

        assert_eq!(error.kind(), clap::error::ErrorKind::InvalidSubcommand);
    }
}
