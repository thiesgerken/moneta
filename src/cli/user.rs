use crate::hash_password;
use crate::models::rendering::RenderedBalance;
use crate::models::rendering::RenderedExpense;
use crate::models::*;
use crate::queries;

use clap::ArgMatches;
use clap::{Arg, ArgGroup, Command};
use futures::TryStreamExt;
use log::info;
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use prettytable::{row, Table};
use std::io;

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
        .arg(Arg::new("list").long("list").help("list users"))
        .group(
            ArgGroup::new("action")
                .args(&["add", "update", "remove", "list"])
                .required(true),
        )
}

pub async fn handle(connection: mongodb::Database, sub_matches: &ArgMatches) {
    let user_collection = connection.collection::<User>("users");

    if let Some(uname) = sub_matches.get_one::<String>("add") {
        // check if this username is taken
        if let Some(user) = queries::query_user_by_name(&connection, uname)
            .await
            .expect("Cannot query users")
        {
            panic!(
                "there is already a user (_id: {}) with name '{}'!",
                *user.id, uname
            );
        }

        let mut fname = String::new();
        println!("Please enter the full name of the new user");
        io::stdin().read_line(&mut fname).unwrap();
        let fname = fname.trim_end(); // Remove the trailing newline

        let mut password = String::new();
        println!("Please enter a password for the new user");
        io::stdin().read_line(&mut password).unwrap();
        let password = password.trim_end(); // Remove the trailing newline

        assert!(
            password.len() >= 8,
            "Password has to be at least 8 characters long!"
        );

        let u = User {
            id: ObjectIdWrapper(ObjectId::new()),
            name: uname.to_string(),
            full_name: fname.to_string(),
            password_hash: hash_password(password),
        };

        user_collection
            .insert_one(&u, None)
            .await
            .expect("Error saving new user");
        info!("Created user {:?}", u);
    } else if let Some(uname) = sub_matches.get_one::<String>("update") {
        // check if this user even exists
        let user = queries::query_user_by_name(&connection, uname)
            .await
            .expect("Error loading users")
            .expect("User does not exist!");

        let mut fname = String::new();
        println!(
            "Please enter a new full name for the user {} (current: {})",
            user.name, user.full_name
        );
        io::stdin().read_line(&mut fname).unwrap();
        let fname = fname.trim_end(); // Remove the trailing newline

        let mut password = String::new();
        println!("Please enter a new password for the user {}", user.name);
        io::stdin().read_line(&mut password).unwrap();
        let password = password.trim_end(); // Remove the trailing newline

        assert!(
            password.len() >= 8,
            "Password has to be at least 8 characters long!"
        );

        user_collection
            .update_one(
                doc! {"_id": *user.id},
                doc! {"$set": {"fullName":fname, "passwordHash": hash_password(password)}},
                None,
            )
            .await
            .expect("Could not update user");
        info!("Updated user {}", uname);
    } else if let Some(uname) = sub_matches.get_one::<String>("remove") {
        // check if this user even exists
        let user = queries::query_user_by_name(&connection, uname)
            .await
            .expect("Error loading users")
            .expect("User does not exist!");

        let mut confirmation = String::new();
        println!(
          "Do you really want to delete this user? If so, enter '{}'.\n Note that this will also delete relevant accounts and expenses!",
          uname
              );
        io::stdin().read_line(&mut confirmation).unwrap();
        let confirmation = confirmation.trim_end(); // Remove the trailing newline
        assert_eq!(confirmation, uname, "Confirmation failed");

        // TODO: also need to remove orphaned accounts, expenses, ..., re-render stuff, ...
        let delete_res = user_collection
            .delete_one(doc! {"_id": *user.id}, None)
            .await
            .expect("Could not delete user");

        assert!(delete_res.deleted_count == 1);
        info!("deleted user '{}'", uname);
    } else if sub_matches.contains_id("list") {
        let mut user_cursor = user_collection
            .find(None, None)
            .await
            .expect("Could not obtain users");

        let mut table = Table::new();
        table.add_row(row![
            "ID",
            "Name",
            "Full Name",
            "# Accounts",
            "# Categories",
            "# Expenses",
            "# Balances"
        ]);

        while let Some(u) = user_cursor.try_next().await.expect("error loading user") {
            let r_expense_collection =
                connection.collection::<RenderedExpense>("expenses_rendered");
            let r_expense_count = r_expense_collection
                .count_documents(doc! {"userId": *u.id}, None)
                .await
                .expect("Could not obtain expenses of user");

            let r_balance_collection =
                connection.collection::<RenderedBalance>("balances_rendered");
            let r_balance_count = r_balance_collection
                .count_documents(doc! {"userId": *u.id}, None)
                .await
                .expect("Could not obtain balances of user");

            let account_collection = connection.collection::<Account>("accounts");
            let account_count = account_collection
                .count_documents(doc! {"userId": *u.id}, None)
                .await
                .expect("Could not obtain accounts of user");

            let category_collection = connection.collection::<Category>("categories");
            let category_count = category_collection
                .count_documents(doc! {"userId": *u.id}, None)
                .await
                .expect("Could not obtain categories of user");

            table.add_row(row![
                u.id,
                u.name,
                u.full_name,
                account_count,
                category_count,
                r_expense_count,
                r_balance_count
            ]);
        }

        table.printstd(); // cSpell: disable-line
    } else {
        panic!("unexpected options for subcommand 'user'");
    }
}
