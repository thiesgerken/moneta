use crate::models::PeriodicRule;
use crate::queries;
use crate::rules::{PERIODIC_ADVANCE, generate_periodic_expenses, occurrences};
use crate::schema::periodic_rules;

use chrono::Utc;
use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use log::info;
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

pub async fn handle(pool: crate::Pool, sub_matches: &ArgMatches) {
    let conn = &mut pool.get().await.expect("Could not get a connection");

    if sub_matches.get_flag("run") {
        let count = generate_periodic_expenses(conn, Utc::now())
            .await
            .expect("could not generate periodic expenses");

        info!("generated {} expense(s) from periodic rules", count);
        return;
    }

    let rules: Vec<PeriodicRule> = periodic_rules::table
        .order_by(periodic_rules::title.asc())
        .select(PeriodicRule::as_select())
        .load(conn)
        .await
        .expect("could not load periodic rules");

    let until = Utc::now() + PERIODIC_ADVANCE;

    let mut table = Table::new();
    table.add_row(row!["ID", "Title", "Period", "Last creation", "Due"]);

    for rule in &rules {
        let template = queries::load_expense(conn, rule.template_id)
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
