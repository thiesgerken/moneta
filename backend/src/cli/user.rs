use std::io;

use crate::hash_password;
use crate::models::User;
use crate::queries;
use crate::schema::{accounts, users};
use crate::schema_views::{balances_rendered, expenses_rendered};

use clap::ArgMatches;
use clap::{Arg, ArgAction, ArgGroup, Command};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use log::info;
use prettytable::{Table, row};
use uuid::Uuid;

pub fn build() -> Command {
    Command::new("user")
        .about("User Management")
        .arg(
            Arg::new("add")
                .long("add")
                .value_name("name")
                .help("add user"),
        )
        .arg(
            Arg::new("remove")
                .long("remove")
                .value_name("name")
                .help("remove user"),
        )
        .arg(
            Arg::new("update")
                .long("update")
                .value_name("name")
                .help("update user name and password"),
        )
        .arg(
            Arg::new("stockdb-token")
                .long("stockdb-token")
                .value_name("name")
                .help("set this user's stockdb API token, read on stdin (empty clears it)"),
        )
        .arg(
            Arg::new("list")
                .long("list")
                .action(ArgAction::SetTrue)
                .help("list users"),
        )
        .group(
            ArgGroup::new("action")
                .args(["add", "update", "remove", "stockdb-token", "list"])
                .required(true),
        )
}

fn prompt(message: &str) -> String {
    let mut line = String::new();
    println!("{message}");
    io::stdin().read_line(&mut line).unwrap();
    line.trim_end().to_owned()
}

pub async fn handle(pool: crate::Pool, sub_matches: &ArgMatches) {
    let conn = &mut pool.get().await.expect("Could not get a connection");

    if let Some(uname) = sub_matches.get_one::<String>("add") {
        if let Some(user) = queries::query_user_by_name(conn, uname)
            .await
            .expect("Cannot query users")
        {
            panic!(
                "there is already a user ({}) with name '{}'!",
                user.id, uname
            );
        }

        let fname = prompt("Please enter the full name of the new user");
        let password = prompt("Please enter a password for the new user");

        assert!(
            password.len() >= 8,
            "Password has to be at least 8 characters long!"
        );

        let u = User {
            id: Uuid::now_v7(),
            name: uname.to_string(),
            full_name: fname,
            password_hash: hash_password(&password),
        };

        diesel::insert_into(users::table)
            .values(&u)
            .execute(conn)
            .await
            .expect("Error saving new user");
        info!("Created user {} ({})", u.name, u.id);
    } else if let Some(uname) = sub_matches.get_one::<String>("update") {
        let user = queries::query_user_by_name(conn, uname)
            .await
            .expect("Error loading users")
            .expect("User does not exist!");

        let fname = prompt(&format!(
            "Please enter a new full name for the user {} (current: {})",
            user.name, user.full_name
        ));
        let password = prompt(&format!(
            "Please enter a new password for the user {}",
            user.name
        ));

        assert!(
            password.len() >= 8,
            "Password has to be at least 8 characters long!"
        );

        diesel::update(users::table.find(user.id))
            .set((
                users::full_name.eq(fname),
                users::password_hash.eq(hash_password(&password)),
            ))
            .execute(conn)
            .await
            .expect("Could not update user");
        info!("Updated user {}", uname);
    } else if let Some(uname) = sub_matches.get_one::<String>("stockdb-token") {
        let user = queries::query_user_by_name(conn, uname)
            .await
            .expect("Error loading users")
            .expect("User does not exist!");

        // stockdb prints a token once and keeps only its hash, so this is the token itself and
        // there is no way around storing it. It is read here rather than taken as an argument
        // so that it stays out of the shell history and out of `ps`.
        let token = prompt(&format!(
            "Please paste the stockdb API token for {} (`stockdb user --token {}` issues one)",
            user.name, user.name
        ));

        diesel::update(users::table.find(user.id))
            .set(users::stockdb_token.eq(Some(token.clone()).filter(|t| !t.is_empty())))
            .execute(conn)
            .await
            .expect("Could not update user");

        if token.is_empty() {
            info!("Cleared the stockdb token of user {}", uname);
        } else {
            info!("Set the stockdb token of user {}", uname);
        }
    } else if let Some(uname) = sub_matches.get_one::<String>("remove") {
        let user = queries::query_user_by_name(conn, uname)
            .await
            .expect("Error loading users")
            .expect("User does not exist!");

        let confirmation = prompt(&format!(
            "Do you really want to delete this user? If so, enter '{uname}'.\n Note that the accounts foreign key is ON DELETE RESTRICT, so this fails while they still own any."
        ));
        assert_eq!(confirmation, *uname, "Confirmation failed");

        let count = diesel::delete(users::table.find(user.id))
            .execute(conn)
            .await
            .expect("Could not delete user");

        assert!(count == 1);
        info!("deleted user '{}'", uname);
    } else if sub_matches.contains_id("list") {
        // The token itself never leaves the database; this only says whether there is one.
        let found: Vec<(User, Option<String>)> = users::table
            .order_by(users::name.asc())
            .select((User::as_select(), users::stockdb_token))
            .load(conn)
            .await
            .expect("Could not obtain users");

        let mut table = Table::new();
        table.add_row(row![
            "ID",
            "Name",
            "Full Name",
            "stockdb",
            "# Accounts",
            "# Expenses",
            "# Balances"
        ]);

        for (u, stockdb_token) in found {
            let account_count: i64 = accounts::table
                .filter(accounts::user_id.eq(u.id))
                .count()
                .get_result(conn)
                .await
                .expect("Could not obtain accounts of user");

            let expense_count: i64 = expenses_rendered::table
                .filter(expenses_rendered::user_id.eq(u.id))
                .count()
                .get_result(conn)
                .await
                .expect("Could not obtain expenses of user");

            let balance_count: i64 = balances_rendered::table
                .filter(balances_rendered::user_id.eq(u.id))
                .count()
                .get_result(conn)
                .await
                .expect("Could not obtain balances of user");

            table.add_row(row![
                u.id,
                u.name,
                u.full_name,
                if stockdb_token.is_some() {
                    "token"
                } else {
                    "-"
                },
                account_count,
                expense_count,
                balance_count
            ]);
        }

        table.printstd(); // cSpell: disable-line
    } else {
        panic!("unexpected options for subcommand 'user'");
    }
}
