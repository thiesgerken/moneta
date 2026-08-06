//! Account balances computed from the nearest reference balance, at one date or many.
//!
//! This is the one place raw SQL earns its keep: "nearest balance per account, plus every
//! transaction between it and the sampled date" is two `LIMIT 1` lookups and an aggregate,
//! which the mongo version had to do by pulling the expenses over the wire and folding them
//! in Rust. Sampling a whole range is then the same query over an array of dates rather than
//! one round trip per point.

use std::collections::HashMap;

use uuid::Uuid;

use crate::graphql::root::{Context, require_user};
use crate::models::AccountAvailability;
use crate::models::rendering::{RenderedBalance, RenderedExpense};
use crate::schema_views::{balances_rendered, expenses_rendered};

use chrono::DateTime;
use chrono::Utc;
use diesel::prelude::*;
use diesel::sql_types::{Array, BigInt, Double, Nullable, Timestamptz};
use diesel_async::RunQueryDsl;
use itertools::Itertools;
use juniper::{FieldResult, GraphQLObject, graphql_object};

use super::accounts::GraphQLAccount;
use super::balances::GraphQLBalance;
use super::expenses::GraphQLExpense;

#[derive(Debug, Clone, GraphQLObject)]
#[graphql(context = Context)]
pub struct ComputedBalance {
    pub date: DateTime<Utc>,
    pub amount: f64,
    pub reference: GraphQLBalance,
    pub account: GraphQLAccount,
    pub expense_count: i32,
}

/// One account's balance at every sampled date, in the order [`History::dates`] gives them.
#[derive(Debug, Clone, GraphQLObject)]
#[graphql(context = Context)]
pub struct AccountSeries {
    pub account: GraphQLAccount,
    pub amounts: Vec<f64>,
}

/// The accounts of one availability summed together.
///
/// Per bucket rather than cumulative: stacking these gives the cumulative reading back as the
/// height of the stack, whereas stacking cumulative series would count the liquid accounts
/// once per level above them.
#[derive(Debug, Clone, GraphQLObject)]
#[graphql(context = Context)]
pub struct AvailabilitySeries {
    pub availability: AccountAvailability,
    pub amounts: Vec<f64>,
}

#[derive(Debug, Clone, GraphQLObject)]
#[graphql(context = Context)]
pub struct History {
    pub dates: Vec<DateTime<Utc>>,
    pub by_account: Vec<AccountSeries>,
    pub by_availability: Vec<AvailabilitySeries>,
}

/// What is left of an expense whose value is still being consumed at the requested date.
#[derive(Debug, Clone, GraphQLObject)]
#[graphql(context = Context)]
pub struct Prepayment {
    pub expense: GraphQLExpense,
    /// how much of the booking period has elapsed, 0..1
    pub consumed_fraction: f64,
    /// cents not yet consumed, in the same sign as the expense itself
    pub remaining: f64,
}

#[derive(Debug, QueryableByName)]
struct SimulatedRow {
    #[diesel(sql_type = BigInt)]
    sample_idx: i64,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    account_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    balance_id: Uuid,
    #[diesel(sql_type = Double)]
    amount: f64,
    #[diesel(sql_type = BigInt)]
    expense_count: i64,
}

/// The arithmetic itself is `simulate_balances` in the migrations, so that `tests/sql` can
/// reach it; `$1` is the user, `$2` the dates to sample, `$3` an optional single account.
const SIMULATE: &str = r#"
SELECT sample_idx, account_id, balance_id, amount, expense_count
FROM simulate_balances($1, $2::timestamptz[], $3::uuid)
"#;

/// Ceiling on how many dates one request may ask to be simulated.
///
/// `from`/`to`/`stepDays` are user input, and a decade at daily resolution is ~3650 samples.
/// The chart asks for ~120 whatever it is showing, so this is only ever hit by a caller that
/// has lost track of what it is requesting -- worth an error rather than a silently coarser
/// answer that looks like the one that was asked for.
const MAX_SAMPLES: usize = 400;

pub struct AnalysisQuery;

#[graphql_object(
    Context = Context,
)]
impl AnalysisQuery {
    pub async fn balance(
        context: &Context,
        date: Option<DateTime<Utc>>,
        account_id: Uuid,
    ) -> FieldResult<ComputedBalance> {
        let date = date.unwrap_or_else(Utc::now);
        let user_id = require_user(context).await?;

        let account = context.account_loader.try_load(account_id).await?;
        if account.user_id != user_id {
            return Err(format!(
                "Not going to tell you balances for account {} that isn't yours!",
                account.id
            )
            .into());
        }

        compute(context, date, Some(account_id))
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| format!("Account {account_id} has no balance to compute from").into())
    }

    pub async fn balances(
        context: &Context,
        date: Option<DateTime<Utc>>,
    ) -> FieldResult<Vec<ComputedBalance>> {
        let date = date.unwrap_or_else(Utc::now);
        require_user(context).await?;

        Ok(compute(context, date, None)
            .await?
            .into_iter()
            .sorted_by_key(|b| b.account.data.name.clone())
            .collect())
    }

    /// The same computation as [`AnalysisQuery::balances`], repeated across a range.
    ///
    /// `stepDays` comes from the chart's visible range rather than being fixed, so zoomed out
    /// it lands around 30 and zoomed all the way in on 1. `to` is always the last sample even
    /// when the step does not divide the range, otherwise the line stops short of it.
    pub async fn history(
        context: &Context,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        step_days: Option<i32>,
    ) -> FieldResult<History> {
        require_user(context).await?;

        let dates = sample_dates(from, to, step_days.unwrap_or(30))?;
        let rows = simulate(context, &dates, None).await?;

        let accounts = context
            .account_loader
            .load_many(rows.iter().map(|r| r.account_id).unique().collect())
            .await;

        let mut by_account: HashMap<Uuid, Vec<f64>> = HashMap::new();
        for row in &rows {
            let amounts = by_account
                .entry(row.account_id)
                .or_insert_with(|| vec![0.0; dates.len()]);
            // WITH ORDINALITY counts from one
            amounts[row.sample_idx as usize - 1] = row.amount;
        }

        let mut by_availability: HashMap<AccountAvailability, Vec<f64>> = HashMap::new();
        let mut series = Vec::with_capacity(by_account.len());
        for (account_id, amounts) in by_account {
            let account = accounts
                .get(&account_id)
                .ok_or_else(|| format!("Account {account_id} does not exist"))?;

            let bucket = by_availability
                .entry(account.availability)
                .or_insert_with(|| vec![0.0; dates.len()]);
            for (total, amount) in bucket.iter_mut().zip(&amounts) {
                *total += amount;
            }

            series.push(AccountSeries {
                account: GraphQLAccount::new(account.clone()),
                amounts,
            });
        }

        series.sort_by(|a, b| a.account.data.name.cmp(&b.account.data.name));

        Ok(History {
            dates,
            by_account: series,
            // liquid first, so a stack reads from the money you can spend today upwards
            by_availability: AccountAvailability::ORDERED
                .iter()
                .filter_map(|availability| {
                    by_availability
                        .remove(availability)
                        .map(|amounts| AvailabilitySeries {
                            availability: *availability,
                            amounts,
                        })
                })
                .collect(),
        })
    }

    /// Expenses whose value is not yet fully consumed at `at`.
    ///
    /// The booking period is what makes this work without the prepayment accounts and
    /// compensation flags MoneyDB needed: an expense whose money left on the transaction date
    /// but whose value is spread over a period is simply one with `bookingEnd > bookingStart`.
    /// Every imported expense has a zero-length period, so nothing shows up here until a
    /// period is actually widened.
    ///
    /// `isIncomplete` expenses are open items too, but with an unknown missing amount and so
    /// no remaining balance to compute; they come from `expenses.list` with a flag filter
    /// rather than from here.
    pub async fn prepayments(
        context: &Context,
        at: Option<DateTime<Utc>>,
    ) -> FieldResult<Vec<Prepayment>> {
        let at = at.unwrap_or_else(Utc::now);
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let found: Vec<RenderedExpense> = expenses_rendered::table
            .filter(expenses_rendered::user_id.eq(user_id))
            .filter(expenses_rendered::is_template.eq(false))
            // a zero-length period degenerates to an ordinary expense, and `booking_end > at`
            // is what leaves out everything already consumed
            .filter(expenses_rendered::booking_end.gt(expenses_rendered::booking_start))
            .filter(expenses_rendered::booking_end.gt(at))
            .filter(expenses_rendered::effective_amount.ne(0.0))
            .select(RenderedExpense::as_select())
            .load(conn)
            .await?;

        Ok(found
            .into_iter()
            .map(|expense| {
                let consumed_fraction =
                    consumed_fraction(expense.booking_start, expense.booking_end, at);
                Prepayment {
                    remaining: expense.effective_amount * (1.0 - consumed_fraction),
                    consumed_fraction,
                    expense: GraphQLExpense::new(expense),
                }
            })
            // biggest outstanding first; an expense is negative, so that is ascending
            .sorted_by(|a, b| a.remaining.total_cmp(&b.remaining))
            .collect())
    }
}

/// Linear accrual across the booking period, as [`AnalysisQuery::prepayments`] describes.
fn consumed_fraction(start: DateTime<Utc>, end: DateTime<Utc>, at: DateTime<Utc>) -> f64 {
    let span = (end - start).num_milliseconds();
    if span <= 0 {
        // `Expense::validate` rejects a backwards period, so this is the zero-length case:
        // an ordinary expense, consumed the moment it starts
        return if at >= start { 1.0 } else { 0.0 };
    }

    ((at - start).num_milliseconds() as f64 / span as f64).clamp(0.0, 1.0)
}

/// The dates [`AnalysisQuery::history`] samples, always ending exactly on `to`.
fn sample_dates(
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    step_days: i32,
) -> FieldResult<Vec<DateTime<Utc>>> {
    if to < from {
        return Err(format!("History range ends ({to}) before it starts ({from})").into());
    }
    if step_days < 1 {
        return Err(format!("History step has to be at least a day, got {step_days}").into());
    }

    let step = chrono::Duration::days(step_days.into());
    let mut dates = Vec::new();
    let mut at = from;
    while at < to {
        dates.push(at);
        if dates.len() >= MAX_SAMPLES {
            return Err(format!(
                "History from {from} to {to} at a step of {step_days} day(s) needs more than \
                 {MAX_SAMPLES} samples; ask for a coarser step"
            )
            .into());
        }
        at += step;
    }
    dates.push(to);

    Ok(dates)
}

async fn simulate(
    context: &Context,
    dates: &[DateTime<Utc>],
    account_id: Option<Uuid>,
) -> FieldResult<Vec<SimulatedRow>> {
    let user_id = require_user(context).await?;
    let conn = &mut context.conn().await?;

    Ok(diesel::sql_query(SIMULATE)
        .bind::<diesel::sql_types::Uuid, _>(user_id)
        .bind::<Array<Timestamptz>, _>(dates.to_vec())
        .bind::<Nullable<diesel::sql_types::Uuid>, _>(account_id)
        .load(conn)
        .await?)
}

async fn compute(
    context: &Context,
    date: DateTime<Utc>,
    account_id: Option<Uuid>,
) -> FieldResult<Vec<ComputedBalance>> {
    let user_id = require_user(context).await?;
    let rows = simulate(context, &[date], account_id).await?;
    let conn = &mut context.conn().await?;

    let references: Vec<RenderedBalance> = balances_rendered::table
        .filter(balances_rendered::user_id.eq(user_id))
        .filter(balances_rendered::id.eq_any(rows.iter().map(|r| r.balance_id).collect::<Vec<_>>()))
        .select(RenderedBalance::as_select())
        .load(conn)
        .await?;

    let accounts = context
        .account_loader
        .load_many(references.iter().map(|b| b.account_id).unique().collect())
        .await;

    let mut computed = Vec::with_capacity(references.len());
    for reference in references {
        let row = rows
            .iter()
            .find(|r| r.balance_id == reference.id)
            .expect("every reference balance came out of the computation");
        let account = accounts
            .get(&reference.account_id)
            .ok_or_else(|| format!("Account {} does not exist", reference.account_id))?;

        computed.push(ComputedBalance {
            date,
            amount: row.amount,
            expense_count: row.expense_count as i32,
            reference: GraphQLBalance::new(reference.clone()),
            account: GraphQLAccount::new(account.clone()),
        });
    }

    Ok(computed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().into()
    }

    #[test]
    fn a_zero_length_period_is_consumed_the_moment_it_starts() {
        let start = t("2026-01-01T00:00:00Z");
        assert_eq!(
            consumed_fraction(start, start, t("2025-12-31T23:59:59Z")),
            0.0
        );
        assert_eq!(consumed_fraction(start, start, start), 1.0);
        assert_eq!(
            consumed_fraction(start, start, t("2026-06-01T00:00:00Z")),
            1.0
        );
    }

    #[test]
    fn a_period_accrues_linearly_and_clamps_outside_itself() {
        let start = t("2026-01-01T00:00:00Z");
        let end = t("2026-01-11T00:00:00Z");

        assert_eq!(
            consumed_fraction(start, end, t("2025-12-01T00:00:00Z")),
            0.0
        );
        assert_eq!(consumed_fraction(start, end, start), 0.0);
        assert_eq!(
            consumed_fraction(start, end, t("2026-01-03T00:00:00Z")),
            0.2
        );
        assert_eq!(
            consumed_fraction(start, end, t("2026-01-06T00:00:00Z")),
            0.5
        );
        assert_eq!(consumed_fraction(start, end, end), 1.0);
        assert_eq!(
            consumed_fraction(start, end, t("2027-01-01T00:00:00Z")),
            1.0
        );
    }

    #[test]
    fn sampling_always_lands_on_the_range_end() {
        let from = t("2026-01-01T00:00:00Z");
        let to = t("2026-01-08T00:00:00Z");

        // 7 days at a step of 3 does not divide evenly; the last step is short rather than
        // the line stopping two days before the range ends
        let dates = sample_dates(from, to, 3).unwrap();
        assert_eq!(
            dates,
            vec![
                from,
                t("2026-01-04T00:00:00Z"),
                t("2026-01-07T00:00:00Z"),
                to
            ]
        );

        // an exact division must not repeat the endpoint
        assert_eq!(sample_dates(from, to, 7).unwrap(), vec![from, to]);
        // and an empty range is a single sample, not none
        assert_eq!(sample_dates(from, from, 1).unwrap(), vec![from]);
    }

    #[test]
    fn sampling_refuses_what_it_cannot_answer() {
        let from = t("2026-01-01T00:00:00Z");

        assert!(sample_dates(from, t("2025-01-01T00:00:00Z"), 1).is_err());
        assert!(sample_dates(from, t("2026-02-01T00:00:00Z"), 0).is_err());
        // a decade at daily resolution is past the ceiling, and saying so beats quietly
        // returning a coarser series than the caller asked for
        assert!(sample_dates(from, t("2036-01-01T00:00:00Z"), 1).is_err());
        assert!(sample_dates(from, t("2036-01-01T00:00:00Z"), 30).is_ok());
    }
}
