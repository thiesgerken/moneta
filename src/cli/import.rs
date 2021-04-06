use crate::schema::*;
use crate::serialization::*;

use clap::ArgMatches;
use clap::{App, Arg, SubCommand};
use diesel::prelude::*;
use log::{debug, info, warn};
use std::io;

pub fn build() -> App<'static, 'static> {
    SubCommand::with_name("import")
        .about("Importing of data via stdin")
        .arg(
            Arg::with_name("moneydb")
                .long("moneydb")
                .help("import from moneydb json data"),
        )
        .arg(
            Arg::with_name("clean")
                .long("clean")
                .help("clean database contents before importing (use with caution!)"),
        )
}

pub fn handle(connection: &PgConnection, sub_matches: &ArgMatches<'_>) {
    if sub_matches.is_present("clean") {
        diesel::delete(users::table)
            .execute(connection)
            .expect("Unable to delete users!");

        warn!("removed all users and everything belonging to them");
    }

    if sub_matches.is_present("moneydb") {
        let p: MoneyDBFormat =
            serde_json::from_reader(io::stdin()).expect("Could not parse moneydb input");
        debug!("{:?}", p);

        p.write_to(connection);
        info!("Successfully imported moneydb data.")
    } else {
        let p: NativeFormat =
            serde_json::from_reader(io::stdin()).expect("Could not parse native input");
        debug!("{:?}", p);

        p.write_to(connection);
        info!("Successfully imported backed-up data.")
    }
}
