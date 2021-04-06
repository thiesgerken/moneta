use moneta::cli;
use moneta::queries;
use moneta::rendering::RenderedExpense;
use moneta::schema::*;

use criterion::{criterion_group, criterion_main, Criterion};
use diesel::prelude::*;
use log::debug;
use simplelog::{LevelFilter, SimpleLogger, TermLogger, TerminalMode};
use std::time::Duration;

fn run_filter_and_render_naive(connection: &PgConnection) -> Option<Vec<RenderedExpense>> {
    let uid = 1;

    let exps = queries::relevant_expenses(connection, uid, None, None).ok()?;
    let transactions =
        queries::expense_transactions_by_expense_range(connection, uid, None, None).ok()?;
    let categories =
        queries::expense_categories_by_expense_range(connection, uid, None, None).ok()?;
    let receipts = queries::expense_receipts_by_expense_range(connection, uid, None, None).ok()?;
    let events = queries::expense_events_by_expense_range(connection, uid, None, None).ok()?;

    let rexps = exps
        .into_iter()
        .map(|e| {
            RenderedExpense::filter_and_render(
                uid,
                e,
                &transactions,
                &categories,
                &receipts,
                &events,
            )
        })
        .collect::<Vec<RenderedExpense>>();

    Some(rexps)
}

fn run_filter_and_render_optim(connection: &PgConnection) -> Option<Vec<RenderedExpense>> {
    let uid = 1;

    let exps = queries::relevant_expenses(connection, uid, None, None).ok()?;
    let transactions =
        queries::expense_transactions_by_expense_range(connection, uid, None, None).ok()?;
    let categories =
        queries::expense_categories_by_expense_range(connection, uid, None, None).ok()?;
    let receipts = queries::expense_receipts_by_expense_range(connection, uid, None, None).ok()?;
    let events = queries::expense_events_by_expense_range(connection, uid, None, None).ok()?;

    let mut rexps = Vec::with_capacity(exps.len());
    let mut t_i = 0;
    let mut c_i = 0;
    let mut r_i = 0;
    let mut e_i = 0;
    for e in exps.into_iter() {
        while transactions
            .get(t_i)
            .map(|(t, _, _)| e.id > t.expense_id)
            .unwrap_or(false)
        {
            t_i += 1;
        }
        while categories
            .get(c_i)
            .map(|(t, _)| e.id > t.expense_id)
            .unwrap_or(false)
        {
            c_i += 1;
        }
        while receipts
            .get(r_i)
            .map(|t| e.id > t.expense_id)
            .unwrap_or(false)
        {
            r_i += 1;
        }
        while events
            .get(e_i)
            .map(|t| e.id > t.expense_id)
            .unwrap_or(false)
        {
            e_i += 1;
        }

        rexps.push(RenderedExpense::filter_and_render(
            uid,
            e,
            &transactions[t_i..],
            &categories[c_i..],
            &receipts[r_i..],
            &events[e_i..],
        ));
    }

    Some(rexps)
}

fn run_filter_and_render_queries(connection: &PgConnection) -> Option<()> {
    let uid = 1;

    let _exps = queries::relevant_expenses(connection, uid, None, None).ok()?;
    let _transactions =
        queries::expense_transactions_by_expense_range(connection, uid, None, None).ok()?;
    let _categories =
        queries::expense_categories_by_expense_range(connection, uid, None, None).ok()?;
    let _receipts = queries::expense_receipts_by_expense_range(connection, uid, None, None).ok()?;
    let _events = queries::expense_events_by_expense_range(connection, uid, None, None).ok()?;

    Some(())
}

fn count_all_expenses(connection: &PgConnection) -> Option<usize> {
    let q = expenses::table
        .select(diesel::dsl::count(expenses::id))
        .filter(
            expenses::id.eq_any(
                expenses::table
                    .select(expenses::id)
                    // .distinct_on(expenses::id)
                    .left_join(
                        expense_transactions::table
                            .on(expense_transactions::expense_id.eq(expenses::id)),
                    )
                    .left_join(
                        account_synchronizations::table.on(expense_transactions::account_id
                            .eq(account_synchronizations::account1)
                            .or(expense_transactions::account_id
                                .eq(account_synchronizations::account2))),
                    )
                    .left_join(
                        accounts::table.on(accounts::id.eq(expense_transactions::account_id)),
                    )
                    .filter(
                        accounts::user_id
                            .eq(1)
                            .or(account_synchronizations::user1.eq(1))
                            .or(account_synchronizations::user2.eq(1)),
                    ),
            ),
        );

    let all = q.load::<i64>(connection).ok()?;
    Some(all.len())
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut log_config = simplelog::ConfigBuilder::new();
    log_config.set_time_format_str("");

    let level = LevelFilter::Info;
    TermLogger::init(level, log_config.build(), TerminalMode::Mixed)
        .ok()
        .or_else(|| SimpleLogger::init(level, log_config.build()).ok())
        .unwrap();

    let settings = cli::initialize_settings(Some("moneta.toml")).unwrap();
    let config = settings.try_into::<cli::Config>().unwrap();
    let connection = moneta::connect(&config.database).unwrap();
    debug!("config={:?}", &config);

    c.bench_function("RenderedExpense::filter_and_render (optim)", |b| {
        b.iter(|| run_filter_and_render_optim(&connection))
    });
    c.bench_function("RenderedExpense::filter_and_render (naive)", |b| {
        b.iter(|| run_filter_and_render_naive(&connection))
    });
    c.bench_function("RenderedExpense::filter_and_render (queries only)", |b| {
        b.iter(|| run_filter_and_render_queries(&connection))
    });
    c.bench_function("count all expenses", |b| {
        b.iter(|| count_all_expenses(&connection))
    });
}

criterion_group!(
  name = benches;
  config = Criterion::default().sample_size(20).measurement_time(Duration::from_secs(15));
  targets = criterion_benchmark
);
criterion_main!(benches);
