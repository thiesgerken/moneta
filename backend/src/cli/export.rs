use std::io::{self, BufWriter, Write};

use clap::Command;
use log::info;

use crate::backup::Backup;

pub fn build() -> Command {
    Command::new("export")
        .about("write all users' data as JSON to stdout")
        .long_about(
            "Write all users' data as JSON to stdout: users, accounts, categories, expenses \
            with their transactions, balances, rules, seen deliveries and the expense \
            changelog. Credentials are left out: no password hashes, API keys or renditier \
            tokens. `import` reads the file back into an empty database.",
        )
}

pub async fn handle(pool: crate::Pool) {
    let conn = &mut pool.get().await.expect("Could not get a connection");
    let backup = Backup::load(conn).await.expect("Could not read the data");

    let mut out = BufWriter::new(io::stdout().lock());
    serde_json::to_writer_pretty(&mut out, &backup).expect("Could not write the export");
    writeln!(out).expect("Could not write the export");
    out.flush().expect("Could not write the export");

    info!(
        "exported {} users, {} accounts and {} expenses",
        backup.users.len(),
        backup.accounts.len(),
        backup.expenses.len()
    );
}
