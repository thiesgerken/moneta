use crate::models::rendering::{RenderedBalance, RenderedExpense};
use crate::models::*;
use crate::rendering::rerender_multiple;

use clap::ArgMatches;
use clap::{Arg, ArgGroup, Command};
use futures::TryStreamExt;
use log::info;
use mongodb::bson;
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use prettytable::{Table, row};
use std::str::FromStr;

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
        .arg(
            Arg::new("rerender")
                .long("rerender")
                .help("rerender balances and expenses"),
        )
        .arg(Arg::new("list").long("list").help("list accounts"))
        .group(
            ArgGroup::new("action")
                .args(["sync", "list", "unsync", "rerender"])
                .required(true),
        )
}

pub async fn handle(connection: mongodb::Database, sub_matches: &ArgMatches) {
    let account_collection = connection.collection::<Account>("accounts");

    if let Some(account_id_values) = sub_matches.get_many::<String>("sync") {
        assert!(account_id_values.len() == 2);
        let account_ids: Vec<ObjectId> = account_id_values
            .map(|s| ObjectId::from_str(s))
            .collect::<Result<_, _>>()
            .expect("error parsing object ids");

        let accounts: Vec<_> = account_collection
            .find(doc!("_id": {"$in": &account_ids}))
            .await
            .expect("Error loading accounts")
            .try_collect()
            .await
            .expect("Error loading accounts");

        assert!(accounts.len() == 2);
        assert!(accounts.iter().all(|a| a.synchronization.is_none()));
        assert!(accounts[0].user_id != accounts[1].user_id);

        let invert = sub_matches.contains_id("invert");

        for i in 0..2 {
            account_collection
            .update_one(
                doc! {"_id": *accounts[i ].id},
                doc! {"$set": {"synchronization": bson::serialize_to_bson(&Some(AccountSynchronization{ account_id: accounts[(i+1)%2].id, invert })).unwrap()}},

            )
            .await
            .expect("Could not update account");
        }
        info!(
            "Updated accounts {:?} so that they are synchronized, invert={}",
            accounts, invert
        );

        rerender_multiple(connection, &account_ids)
            .await
            .expect("Could not rerender entities");
    } else if let Some(account_id_value) = sub_matches.get_one::<String>("unsync") {
        let account_id = ObjectId::from_str(account_id_value).expect("error parsing object id");
        let account = account_collection
            .find_one(doc!("_id": account_id))
            .await
            .expect("Error loading accounts")
            .expect("Error loading accounts");

        assert!(account.synchronization.is_some());
        let account_ids = vec![account_id, *account.synchronization.unwrap().account_id];

        account_collection
            .update_many(
                doc! {"_id": {"$in":& account_ids}},
                doc! {"$unset": {"synchronization":1}},
            )
            .await
            .expect("Could not update accounts");

        info!("Removed the synchronization of accounts {:?}", account_ids);

        rerender_multiple(connection, &account_ids)
            .await
            .expect("Could not rerender entities");
    } else if sub_matches.contains_id("rerender") {
        let rendered_expenses_collection =
            connection.collection::<RenderedExpense>("expenses_rendered");
        let rendered_balances_collection =
            connection.collection::<RenderedBalance>("balances_rendered");
        let expense_count = rendered_expenses_collection
            .count_documents(doc!())
            .await
            .expect("Error counting rendered expenses");
        let balance_count = rendered_balances_collection
            .count_documents(doc!())
            .await
            .expect("Error counting rendered balances");

        info!("Database has {} rendered balances", balance_count);
        info!("Database has {} rendered expenses", expense_count);

        let accounts: Vec<_> = account_collection
            .find(doc!())
            .await
            .expect("Error loading accounts")
            .try_collect()
            .await
            .expect("Error loading accounts");
        let account_ids: Vec<_> = accounts.iter().map(|a| *a.id).collect();

        rerender_multiple(connection, &account_ids)
            .await
            .expect("Could not rerender entities");
    } else if sub_matches.contains_id("list") {
        let mut account_cursor = account_collection
            .find(doc! {})
            .await
            .expect("Could not obtain users");

        let mut table = Table::new();
        table.add_row(row!["ID", "Name", "Owner", "Synchronization"]);

        while let Some(a) = account_cursor
            .try_next()
            .await
            .expect("error loading accounts")
        {
            table.add_row(row![
                a.id,
                a.name,
                a.user_id,
                format!("{:?}", a.synchronization)
            ]);
        }

        table.printstd(); // cSpell: disable-line
    } else {
        panic!("unexpected options for subcommand 'account'");
    }
}
