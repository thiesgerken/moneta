use std::str::FromStr;

use crate::models::Account;
use crate::schema::accounts;

use clap::ArgMatches;
use clap::{Arg, ArgGroup, Command};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use log::info;
use prettytable::{Table, row};
use uuid::Uuid;

pub fn build() -> Command {
    Command::new("account")
        .about("Account Management")
        .arg(
            Arg::new("sync")
                .long("sync")
                .value_names(["id1", "id2"])
                .number_of_values(2)
                .help("synchronize two accounts"),
        )
        .arg(
            Arg::new("unsync")
                .long("unsync")
                .value_name("id")
                .help("delete the account's synchronization"),
        )
        .arg(
            Arg::new("invert")
                .long("invert")
                .help("for --sync, make it so that the two accounts mirror each other"),
        )
        .arg(Arg::new("list").long("list").help("list accounts"))
        .group(
            ArgGroup::new("action")
                .args(["sync", "list", "unsync"])
                .required(true),
        )
}

pub async fn handle(pool: crate::Pool, sub_matches: &ArgMatches) {
    let conn = &mut pool.get().await.expect("Could not get a connection");

    if let Some(account_id_values) = sub_matches.get_many::<String>("sync") {
        assert!(account_id_values.len() == 2);
        let account_ids: Vec<Uuid> = account_id_values
            .map(|s| Uuid::from_str(s))
            .collect::<Result<_, _>>()
            .expect("error parsing account ids");

        let found: Vec<Account> = accounts::table
            .filter(accounts::id.eq_any(&account_ids))
            .select(Account::as_select())
            .load(conn)
            .await
            .expect("Error loading accounts");

        assert!(found.len() == 2);
        assert!(found.iter().all(|a| a.synchronization().is_none()));
        assert!(found[0].user_id != found[1].user_id);

        let invert = sub_matches.contains_id("invert");

        // Both directions have to be set or neither: the mirror is a pair of rows and no
        // constraint can see across them.
        conn.transaction::<_, diesel::result::Error, _>(async |conn| {
            for i in 0..2 {
                diesel::update(accounts::table.find(found[i].id))
                    .set((
                        accounts::sync_account_id.eq(found[(i + 1) % 2].id),
                        accounts::sync_invert.eq(invert),
                    ))
                    .execute(conn)
                    .await?;
            }
            Ok(())
        })
        .await
        .expect("Could not update accounts");

        info!(
            "Updated accounts {:?} so that they are synchronized, invert={}",
            account_ids, invert
        );
    } else if let Some(account_id_value) = sub_matches.get_one::<String>("unsync") {
        let account_id = Uuid::from_str(account_id_value).expect("error parsing account id");
        let account: Account = accounts::table
            .find(account_id)
            .select(Account::as_select())
            .first(conn)
            .await
            .expect("Error loading account");

        let sync = account
            .synchronization()
            .expect("account is not synchronized");
        let account_ids = vec![account_id, sync.account_id];

        diesel::update(accounts::table.filter(accounts::id.eq_any(&account_ids)))
            .set((
                accounts::sync_account_id.eq(None::<Uuid>),
                accounts::sync_invert.eq(None::<bool>),
            ))
            .execute(conn)
            .await
            .expect("Could not update accounts");

        info!("Removed the synchronization of accounts {:?}", account_ids);
    } else if sub_matches.contains_id("list") {
        let found: Vec<Account> = accounts::table
            .order_by(accounts::name.asc())
            .select(Account::as_select())
            .load(conn)
            .await
            .expect("Could not obtain accounts");

        let mut table = Table::new();
        table.add_row(row!["ID", "Name", "Owner", "Synchronization"]);

        for a in found {
            table.add_row(row![
                a.id,
                a.name,
                a.user_id,
                format!("{:?}", a.synchronization())
            ]);
        }

        table.printstd(); // cSpell: disable-line
    } else {
        panic!("unexpected options for subcommand 'account'");
    }
}
