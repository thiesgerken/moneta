use std::io;

use clap::Command;
use log::{info, warn};

use crate::backup::Backup;

pub fn build() -> Command {
    Command::new("import")
        .about("read an export from stdin into an empty database")
        .long_about(
            "Read an export (as written by `export`) from stdin into a database that holds no \
            user data yet; anything else is refused before the first write. Ids are kept. \
            The file is checked for duplicate ids and dangling references first, and any \
            error rolls the whole import back. Imported users have no password: set one with \
            `moneta user --update <name>`.",
        )
}

pub async fn handle(pool: crate::Pool) {
    let backup = match Backup::read(io::BufReader::new(io::stdin().lock())) {
        Ok(backup) => backup,
        Err(e) => panic!("Could not read the export, nothing was written: {e}"),
    };

    let conn = &mut pool.get().await.expect("Could not get a connection");
    if let Err(e) = backup.restore(conn).await {
        panic!("Could not import, nothing was written: {e}");
    }

    info!(
        "imported {} users, {} accounts and {} expenses",
        backup.users.len(),
        backup.accounts.len(),
        backup.expenses.len()
    );
    warn!("Imported users have no password yet; set one with `moneta user --update <name>`.");
}
