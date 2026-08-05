use crate::models::{Expense, PeriodicRule};
use crate::rules::{PERIODIC_ADVANCE, generate_periodic_expenses, occurrences};

use chrono::Utc;
use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command};
use log::info;
use mongodb::bson::doc;
use prettytable::{Table, row};

pub fn build() -> Command {
    Command::new("periodic")
        .about("Recurring expense rules")
        .arg(
            Arg::new("list")
                .long("list")
                .action(ArgAction::SetTrue)
                .help("list rules and how many expenses each one is due for"),
        )
        .arg(
            Arg::new("run")
                .long("run")
                .action(ArgAction::SetTrue)
                .help("generate the expenses that are due"),
        )
        .group(ArgGroup::new("action").args(["list", "run"]).required(true))
}

pub async fn handle(connection: mongodb::Database, sub_matches: &ArgMatches) {
    if sub_matches.get_flag("run") {
        let count = generate_periodic_expenses(&connection, Utc::now())
            .await
            .expect("could not generate periodic expenses");

        info!("generated {} expense(s) from periodic rules", count);
        return;
    }

    let rules = connection
        .collection::<PeriodicRule>("periodic_rules")
        .find(doc!())
        .await
        .expect("could not load periodic rules")
        .try_collect::<Vec<_>>()
        .await
        .expect("could not load periodic rules");

    let expense_collection = connection.collection::<Expense>("expenses");
    let until = Utc::now() + PERIODIC_ADVANCE;

    let mut table = Table::new();
    table.add_row(row!["ID", "Title", "Period", "Last creation", "Due"]);

    for rule in &rules {
        let template = expense_collection
            .find_one(doc!("_id": *rule.template_id))
            .await
            .expect("could not load template expense");

        let due = match &template {
            Some(t) => occurrences(rule, t, until).len().to_string(),
            None => "template missing".to_owned(),
        };

        table.add_row(row![
            rule.id,
            rule.title,
            format!("{:?}", rule.period),
            rule.last_creation.format("%Y-%m-%d"),
            due
        ]);
    }

    table.printstd(); // cSpell: disable-line
}
