//! Account balances computed from the nearest reference balance, at one date or many.
//!
//! This is the one place raw SQL earns its keep: "nearest balance per account, plus every
//! transaction between it and the sampled date" is two `LIMIT 1` lookups and an aggregate,
//! which the mongo version had to do by pulling the expenses over the wire and folding them
//! in Rust. Sampling a whole range is then the same query over an array of dates rather than
//! one round trip per point.

use std::collections::HashMap;

use uuid::Uuid;

use crate::graphql::cache::{bucket, cached};
use crate::graphql::root::{Context, require_user};
use crate::graphql::scoped;
use crate::models::AccountAvailability;
use crate::models::rendering::{RenderedBalance, RenderedExpense};
use crate::schema_views::{balances_rendered, expenses_rendered};

use chrono::DateTime;
use chrono::Datelike;
use chrono::Utc;
use diesel::prelude::*;
use diesel::sql_types::{Array, BigInt, Bool, Double, Integer, Nullable, Text, Timestamptz};
use diesel_async::RunQueryDsl;
use itertools::Itertools;
use juniper::{Executor, FieldResult, GraphQLObject, ScalarValue, graphql_object};

use super::accounts::GraphQLAccount;
use super::balances::GraphQLBalance;
use super::categories::GraphQLCategory;
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
    /// Everything owned, read on the accrual axis instead: what had been paid for but not yet
    /// used counts as wealth, and what had been used but not yet paid for is already gone.
    ///
    /// Only as a total. Per account it would be an amount no bank ever held, sitting beside
    /// the recorded balances it is supposed to reconcile with — see `accrual_adjustment` in
    /// the migrations. Outside every open accrual period it equals the sum of `byAccount`, so
    /// the two curves are one line wherever nothing is in flight.
    pub accrued_total: Vec<f64>,
}

/// What is left of an expense whose value is still being consumed at the requested date.
#[derive(Debug, Clone, GraphQLObject)]
#[graphql(context = Context)]
pub struct Prepayment {
    pub expense: GraphQLExpense,
    /// how much of the accrual period has elapsed, 0..1
    pub consumed_fraction: f64,
    /// cents not yet consumed, in the same sign as the expense itself
    pub remaining: f64,
}

/// What one category absorbed over a period.
///
/// An expense split across categories contributes its weighted share to each, so these sum to
/// the period's total. `category` is null for the expenses that carry no categorization at
/// all — dropping them would make the parts add up to less than the whole.
#[derive(Debug, Clone, GraphQLObject)]
#[graphql(context = Context)]
pub struct CategoryTotal {
    pub category: Option<GraphQLCategory>,
    pub income: f64,
    pub expenses: f64,
}

/// A category with the individual expenses behind it, for the treemap's inner level.
#[derive(Debug, Clone, GraphQLObject)]
#[graphql(context = Context)]
pub struct CategoryBreakdown {
    pub category: Option<GraphQLCategory>,
    pub income: f64,
    pub expenses: f64,
    /// the biggest by absolute amount, so one call serves both the income and the spending
    /// view; a category holding both signs shows fewer of each
    pub biggest: Vec<CategoryExpense>,
    /// everything `biggest` left out, as one slice, so the treemap still sums to the total
    pub rest_income: f64,
    pub rest_expenses: f64,
}

#[derive(Debug, Clone, GraphQLObject)]
#[graphql(context = Context)]
pub struct CategoryExpense {
    pub expense_id: Uuid,
    pub title: String,
    /// this expense's weighted share of the category, in cents
    pub amount: f64,
}

/// One account's movement across a period.
#[derive(Debug, Clone, GraphQLObject)]
#[graphql(context = Context)]
pub struct AccountOverview {
    pub account: GraphQLAccount,
    pub start_amount: f64,
    pub end_amount: f64,
    /// `startAmount` plus the period's transactions — see [`PeriodOverview::integrated_end_total`]
    pub integrated_end_amount: Option<f64>,
    pub income: f64,
    pub expenses: f64,
    pub expense_count: i32,
}

#[derive(Debug, Clone, GraphQLObject)]
#[graphql(context = Context)]
pub struct PeriodOverview {
    /// midnight on the first day of the period, UTC — the 1st of a month or of a January
    pub start: DateTime<Utc>,
    pub start_total: f64,
    /// what the recorded balances say at the end of the month
    pub end_total: f64,
    /// `startTotal` plus everything booked during the period.
    ///
    /// The gap to `endTotal` is MoneyDB's *Integrationsfehler*. It does not mean the
    /// arithmetic drifted — it means an expense is missing, which is the whole point of
    /// showing it.
    ///
    /// Null when the totals are read on the accrual axis: a sixth of a subscription is a cost
    /// the period carried, not money that left an account, so there is nothing for the
    /// recorded balances to disagree with.
    pub integrated_end_total: Option<f64>,
    pub income: f64,
    pub expenses: f64,
    pub expense_count: i32,
    pub by_account: Vec<AccountOverview>,
    pub by_category: Vec<CategoryTotal>,
}

#[derive(Debug, Clone, GraphQLObject)]
#[graphql(context = Context)]
pub struct Overviews {
    pub periods: Vec<PeriodOverview>,
    /// the whole range at once, for the treemap
    pub by_category: Vec<CategoryBreakdown>,
}

/// How wide one bar of an overview is.
///
/// It is also the word `date_trunc` and `generate_series` take, which is why there are two of
/// them and not a number of days: a month is not a fixed length and neither is a year.
#[derive(Debug, Clone, Copy, PartialEq, Eq, juniper::GraphQLEnum)]
pub enum Granularity {
    Month,
    Year,
}

impl Granularity {
    /// what postgres calls it
    fn grain(self) -> &'static str {
        match self {
            Self::Month => "month",
            Self::Year => "year",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, juniper::GraphQLEnum)]
pub enum StatementEntryKind {
    /// the computed balance the range opens on
    Opening,
    Expense,
    /// a balance that was actually recorded, which re-anchors the running total
    Balance,
}

#[derive(Debug, Clone, GraphQLObject)]
#[graphql(context = Context)]
pub struct StatementEntry {
    pub kind: StatementEntryKind,
    pub date: DateTime<Utc>,
    pub title: String,
    pub store: String,
    pub expense_id: Option<Uuid>,
    /// this account's share, in cents; null on the rows that only assert a balance
    pub amount: Option<f64>,
    /// running total, re-anchored whenever a recorded balance disagrees with it
    pub balance: f64,
    /// on a recorded balance, what it says minus what was carried into it. A non-zero value
    /// is the signal that an expense is missing.
    pub drift: Option<f64>,
}

#[derive(Debug, QueryableByName)]
struct SimulatedRow {
    #[diesel(sql_type = BigInt)]
    sample_idx: i64,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    account_id: Uuid,
    /// null where the anchor is the empty balance an account starts from rather than a
    /// recorded one, and so has nothing for a drift to be measured against
    #[diesel(sql_type = Nullable<diesel::sql_types::Uuid>)]
    balance_id: Option<Uuid>,
    #[diesel(sql_type = Double)]
    amount: f64,
}

#[derive(Debug, QueryableByName)]
struct AdjustmentRow {
    #[diesel(sql_type = BigInt)]
    sample_idx: i64,
    #[diesel(sql_type = Double)]
    amount: f64,
}

#[derive(Debug, QueryableByName)]
struct ExpenseCountRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    account_id: Uuid,
    #[diesel(sql_type = BigInt)]
    expense_count: i64,
}

/// The arithmetic itself is `simulate_balances` in the migrations, so that `tests/sql` can
/// reach it; `$1` is the user, `$2` the dates to sample, `$3` an optional single account.
const SIMULATE: &str = r#"
SELECT sample_idx, account_id, balance_id, amount
FROM simulate_balances($1, $2::timestamptz[], $3::uuid)
"#;

/// What to add to the value-axis total at each sample to read it on the accrual axis instead;
/// the arithmetic and the reasoning are in the migration that defines the function.
const ACCRUAL_ADJUSTMENT: &str = r#"
SELECT sample_idx, amount
FROM accrual_adjustment($1, $2::timestamptz[])
"#;

/// Deliberately not part of `simulate_balances`: counting distinct expenses forces a sort of
/// every transaction in every anchor's interval, which across a chart's worth of samples cost
/// more than everything else together. Asked for one date at a time, where it is cheap.
const COUNT_EXPENSES: &str = r#"
SELECT account_id, expense_count
FROM count_balance_expenses($1, $2, $3::uuid)
"#;

#[derive(Debug, QueryableByName)]
struct PeriodTotalRow {
    #[diesel(sql_type = Timestamptz)]
    period: DateTime<Utc>,
    /// null on the period's roll-up row, which is the only place the expense count is a
    /// distinct count across accounts rather than within one
    #[diesel(sql_type = Nullable<diesel::sql_types::Uuid>)]
    account_id: Option<Uuid>,
    #[diesel(sql_type = Double)]
    income: f64,
    #[diesel(sql_type = Double)]
    expenses: f64,
    #[diesel(sql_type = BigInt)]
    expense_count: i64,
}

#[derive(Debug, QueryableByName)]
struct CategoryTotalRow {
    #[diesel(sql_type = Timestamptz)]
    period: DateTime<Utc>,
    #[diesel(sql_type = Nullable<diesel::sql_types::Uuid>)]
    category_id: Option<Uuid>,
    #[diesel(sql_type = Double)]
    income: f64,
    #[diesel(sql_type = Double)]
    expenses: f64,
}

#[derive(Debug, QueryableByName)]
struct CategoryExpenseRow {
    #[diesel(sql_type = Nullable<diesel::sql_types::Uuid>)]
    category_id: Option<Uuid>,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    expense_id: Uuid,
    #[diesel(sql_type = Text)]
    title: String,
    #[diesel(sql_type = Double)]
    amount: f64,
}

#[derive(Debug, QueryableByName)]
struct StatementRow {
    #[diesel(sql_type = Text)]
    kind: String,
    #[diesel(sql_type = Timestamptz)]
    entry_date: DateTime<Utc>,
    #[diesel(sql_type = Text)]
    title: String,
    #[diesel(sql_type = Text)]
    store: String,
    #[diesel(sql_type = Nullable<diesel::sql_types::Uuid>)]
    expense_id: Option<Uuid>,
    #[diesel(sql_type = Nullable<Double>)]
    amount: Option<f64>,
    #[diesel(sql_type = Double)]
    balance: f64,
    #[diesel(sql_type = Nullable<Double>)]
    drift: Option<f64>,
}

const PERIOD_TOTALS: &str = r#"
SELECT period, account_id, income, expenses, expense_count
FROM analysis_period_totals($1, $2, $3, $4, $5)
"#;

const CATEGORY_TOTALS: &str = r#"
SELECT period, category_id, income, expenses
FROM analysis_category_totals($1, $2, $3, $4, $5)
"#;

const CATEGORY_EXPENSES: &str = r#"
SELECT category_id, expense_id, title, amount
FROM analysis_category_expenses($1, $2, $3, $4, $5)
"#;

const STATEMENT: &str = r#"
SELECT kind, entry_date, title, store, expense_id, amount, balance, drift
FROM analysis_statement($1, $2, $3, $4)
"#;

/// How many expenses the category charts hold inside one category before the remainder
/// collapses into a single slice.
///
/// A dozen was what fitted when the treemap drew every expense at once. It zooms now, and a
/// category that fills the canvas has room for its own long tail -- which is also the tail
/// that makes the Rest slice honest rather than the largest thing in the picture. Fifty is
/// well under what a month holds in one category, and the query is a lateral limit per
/// category rather than a scan.
const TREEMAP_EXPENSES_PER_CATEGORY: i32 = 50;

/// Ceiling on how many periods one `overviews` call may cover.
///
/// Each one costs a balance simulation at its start plus one at the range end, and the table
/// is read a period at a time; a request for six hundred of them is a caller that has lost
/// track, whether that is fifty years of months or six centuries of years.
const MAX_PERIODS: usize = 600;

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
        let user_id = require_user(context).await?;

        cached(
            &context.cache,
            format!("balances:{user_id}:{}", bucket(date)),
            async {
                Ok(compute(context, date, None)
                    .await?
                    .into_iter()
                    .sorted_by_key(|b| b.account.data.name.clone())
                    .collect())
            },
        )
        .await
    }

    /// The same computation as [`AnalysisQuery::balances`], repeated across a range.
    ///
    /// `stepDays` comes from the chart's visible range rather than being fixed, so zoomed out
    /// it lands around 30 and zoomed all the way in on 1. `to` is always the last sample even
    /// when the step does not divide the range, otherwise the line stops short of it.
    ///
    /// `accruedTotal` costs a second statement over the accrual periods, and the start page's
    /// sparkline asks for this endpoint without being able to draw it. So the selection
    /// decides: the correction is computed only for a query that names the field, and left
    /// empty for one that does not. Not observable from a query that did not ask — except
    /// through an alias, where an empty series at least reads as nothing rather than as a
    /// wrong number. It is part of the cache key for the same reason: an entry filled in for
    /// the sparkline must not answer the chart.
    pub async fn history<S: ScalarValue>(
        context: &Context,
        executor: &Executor<'_, '_, Context, S>,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        step_days: Option<i32>,
    ) -> FieldResult<History> {
        let user_id = require_user(context).await?;
        let step_days = step_days.unwrap_or(30);
        let accrued = executor.look_ahead().children().has_child("accruedTotal");

        cached(
            &context.cache,
            format!(
                "history:{user_id}:{}:{}:{step_days}:{accrued}",
                bucket(from),
                bucket(to)
            ),
            history_uncached(context, from, to, step_days, accrued),
        )
        .await
    }

    /// Expenses whose value is not yet fully consumed at `at`.
    ///
    /// The accrual period is what makes this work without the prepayment accounts and
    /// compensation flags MoneyDB needed: an expense whose money left on the transaction date
    /// but whose value is spread over a period is simply one with `accrualEnd > accrualStart`.
    /// The import derives such periods for the historical data rather than leaving everything
    /// degenerate, so this is populated straight after one.
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

        let found: Vec<RenderedExpense> = scoped::expenses(user_id)
            .filter(expenses_rendered::is_template.eq(false))
            // a zero-length period degenerates to an ordinary expense, and `accrual_end > at`
            // is what leaves out everything already consumed
            .filter(expenses_rendered::accrual_end.gt(expenses_rendered::accrual_start))
            .filter(expenses_rendered::accrual_end.gt(at))
            .filter(expenses_rendered::effective_amount.ne(0.0))
            .select(RenderedExpense::as_select())
            .load(conn)
            .await?;

        Ok(found
            .into_iter()
            .map(|expense| {
                let consumed_fraction =
                    consumed_fraction(expense.accrual_start, expense.accrual_end, at);
                Prepayment {
                    remaining: expense.effective_amount * (1.0 - consumed_fraction),
                    consumed_fraction,
                    expense: GraphQLExpense::new(expense),
                }
            })
            // most still tied up first, by size rather than by sign: an expense is negative and
            // an income positive, and what is interesting about either is how much of it is
            // still to come
            .sorted_by(|a, b| b.remaining.abs().total_cmp(&a.remaining.abs()))
            .collect())
    }

    /// Period-by-period income, spending and wealth across a range, a month or a year at a
    /// time.
    ///
    /// `from` and `to` are snapped outwards to whole periods, so asking for the middle of one
    /// gives you that whole period rather than a fragment of it.
    ///
    /// Amounts are `calculatedAmount` — the movement on the account — rather than
    /// `effectiveAmount`. The totals have to reconcile with the balances, and those are
    /// account balances rather than ownership-weighted shares of them; mixing the two would
    /// make the integration error meaningless.
    /// `accrued` files each cost in the periods it was *for* rather than the one it was
    /// paid in, spread evenly over the days of its Leistungszeitraum. The balances stay where
    /// they are either way -- they are money in an account, which an accrued share is not --
    /// so `integratedEndTotal` comes back null and there is no integration error to report.
    pub async fn overviews(
        context: &Context,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        accrued: Option<bool>,
        granularity: Option<Granularity>,
    ) -> FieldResult<Overviews> {
        let user_id = require_user(context).await?;
        let accrued = accrued.unwrap_or(false);
        let granularity = granularity.unwrap_or(Granularity::Month);

        cached(
            &context.cache,
            format!(
                "overviews:{user_id}:{}:{}:{accrued}:{}",
                bucket(from),
                bucket(to),
                granularity.grain()
            ),
            overviews_uncached(context, user_id, from, to, accrued, granularity),
        )
        .await
    }

    /// One account's ledger over a range, as a bank statement reads it.
    ///
    /// Not the expense list with a filter: the computed opening balance, the expenses and the
    /// balances actually recorded are interleaved into one sequence, and a recorded balance
    /// re-anchors the running total. Where it disagrees with the total carried into it, the
    /// gap comes back as `drift` — that is the signal that an expense is missing.
    pub async fn statement(
        context: &Context,
        account_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> FieldResult<Vec<StatementEntry>> {
        let user_id = require_user(context).await?;

        let account = context.account_loader.try_load(account_id).await?;
        if account.user_id != user_id {
            return Err(format!(
                "Not going to tell you the statement for account {} that isn't yours!",
                account.id
            )
            .into());
        }

        if to < from {
            return Err(format!("Statement range ends ({to}) before it starts ({from})").into());
        }

        let conn = &mut context.conn().await?;
        let rows: Vec<StatementRow> = diesel::sql_query(STATEMENT)
            .bind::<diesel::sql_types::Uuid, _>(user_id)
            .bind::<diesel::sql_types::Uuid, _>(account_id)
            .bind::<Timestamptz, _>(from)
            .bind::<Timestamptz, _>(to)
            .load(conn)
            .await?;

        rows.into_iter()
            .map(|row| {
                Ok(StatementEntry {
                    kind: match row.kind.as_str() {
                        "opening" => StatementEntryKind::Opening,
                        "balance" => StatementEntryKind::Balance,
                        "expense" => StatementEntryKind::Expense,
                        other => return Err(format!("Unknown statement entry kind {other}").into()),
                    },
                    date: row.entry_date,
                    title: row.title,
                    store: row.store,
                    expense_id: row.expense_id,
                    amount: row.amount,
                    balance: row.balance,
                    drift: row.drift,
                })
            })
            .collect()
    }
}

/// The body of [`AnalysisQuery::overviews`], outside the resolver so the cache can hold the
/// future without running it.
async fn overviews_uncached(
    context: &Context,
    user_id: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    accrued: bool,
    granularity: Granularity,
) -> FieldResult<Overviews> {
    // one more boundary than there are periods: period i runs [starts[i], starts[i + 1])
    let starts = period_starts(from, to, granularity)?;
    let (range_from, range_to) = (starts[0], starts[starts.len() - 1]);

    let simulated = simulate(context, &starts, None).await?;

    // see `compute`: the connection goes back before the loaders take one of their own
    let (period_rows, category_rows, expense_rows) = {
        let conn = &mut context.conn().await?;

        let period_rows: Vec<PeriodTotalRow> = diesel::sql_query(PERIOD_TOTALS)
            .bind::<diesel::sql_types::Uuid, _>(user_id)
            .bind::<Timestamptz, _>(range_from)
            .bind::<Timestamptz, _>(range_to)
            .bind::<Bool, _>(accrued)
            .bind::<Text, _>(granularity.grain())
            .load(conn)
            .await?;

        let category_rows: Vec<CategoryTotalRow> = diesel::sql_query(CATEGORY_TOTALS)
            .bind::<diesel::sql_types::Uuid, _>(user_id)
            .bind::<Timestamptz, _>(range_from)
            .bind::<Timestamptz, _>(range_to)
            .bind::<Bool, _>(accrued)
            .bind::<Text, _>(granularity.grain())
            .load(conn)
            .await?;

        let expense_rows: Vec<CategoryExpenseRow> = diesel::sql_query(CATEGORY_EXPENSES)
            .bind::<diesel::sql_types::Uuid, _>(user_id)
            .bind::<Timestamptz, _>(range_from)
            .bind::<Timestamptz, _>(range_to)
            .bind::<Integer, _>(TREEMAP_EXPENSES_PER_CATEGORY)
            .bind::<Bool, _>(accrued)
            .load(conn)
            .await?;

        (period_rows, category_rows, expense_rows)
    };

    let accounts = context
        .account_loader
        .load_many(simulated.iter().map(|r| r.account_id).unique().collect())
        .await;
    let categories = context
        .category_loader
        .load_many(
            category_rows
                .iter()
                .filter_map(|r| r.category_id)
                .unique()
                .collect(),
        )
        .await;

    // (sample index, account) -> the balance there, so a period can look up both its ends
    let mut balances: HashMap<(usize, Uuid), f64> = HashMap::new();
    for row in &simulated {
        balances.insert((row.sample_idx as usize - 1, row.account_id), row.amount);
    }

    let mut periods = Vec::with_capacity(starts.len() - 1);
    for (index, start) in starts.iter().take(starts.len() - 1).enumerate() {
        let mut by_account = Vec::new();

        // The roll-up row, whose count is distinct across the period rather than within one
        // account -- summing the per-account counts would count an expense that books
        // against two of your own accounts twice.
        let rolled_up = period_rows
            .iter()
            .find(|r| r.period == *start && r.account_id.is_none());
        let income = rolled_up.map_or(0.0, |t| t.income);
        let expenses = rolled_up.map_or(0.0, |t| t.expenses);
        let expense_count = rolled_up.map_or(0, |t| t.expense_count);

        for (account_id, account) in accounts.iter().sorted_by_key(|(_, a)| a.name.clone()) {
            let totals = period_rows
                .iter()
                .find(|r| r.period == *start && r.account_id == Some(*account_id));

            let start_amount = *balances.get(&(index, *account_id)).unwrap_or(&0.0);
            let end_amount = *balances.get(&(index + 1, *account_id)).unwrap_or(&0.0);
            let moved = totals.map_or(0.0, |t| t.income + t.expenses);

            by_account.push(AccountOverview {
                account: GraphQLAccount::new(account.clone()),
                start_amount,
                end_amount,
                integrated_end_amount: (!accrued).then_some(start_amount + moved),
                income: totals.map_or(0.0, |t| t.income),
                expenses: totals.map_or(0.0, |t| t.expenses),
                expense_count: totals.map_or(0, |t| t.expense_count) as i32,
            });
        }

        let start_total: f64 = by_account.iter().map(|a| a.start_amount).sum();
        let end_total: f64 = by_account.iter().map(|a| a.end_amount).sum();

        periods.push(PeriodOverview {
            start: *start,
            start_total,
            end_total,
            integrated_end_total: (!accrued).then_some(start_total + income + expenses),
            income,
            expenses,
            expense_count: expense_count as i32,
            by_account,
            by_category: category_rows
                .iter()
                .filter(|r| r.period == *start)
                .map(|r| CategoryTotal {
                    category: r
                        .category_id
                        .and_then(|id| categories.get(&id))
                        .map(|c| GraphQLCategory::new(c.clone())),
                    income: r.income,
                    expenses: r.expenses,
                })
                .sorted_by(|a, b| a.expenses.total_cmp(&b.expenses))
                .collect(),
        });
    }

    // whole range at once for the treemap, which is not a sum of the per-period ones: the
    // biggest expenses of a decade are not the biggest of each of its years
    let mut by_category: Vec<CategoryBreakdown> = category_rows
        .iter()
        .map(|r| r.category_id)
        .unique()
        .map(|category_id| {
            let (income, expenses) = category_rows
                .iter()
                .filter(|r| r.category_id == category_id)
                .fold((0.0, 0.0), |(i, e), r| (i + r.income, e + r.expenses));

            let biggest: Vec<CategoryExpense> = expense_rows
                .iter()
                .filter(|r| r.category_id == category_id)
                .map(|r| CategoryExpense {
                    expense_id: r.expense_id,
                    title: r.title.clone(),
                    amount: r.amount,
                })
                .collect();

            CategoryBreakdown {
                category: category_id
                    .and_then(|id| categories.get(&id))
                    .map(|c| GraphQLCategory::new(c.clone())),
                income,
                expenses,
                rest_income: income
                    - biggest
                        .iter()
                        .filter(|e| e.amount > 0.0)
                        .map(|e| e.amount)
                        .sum::<f64>(),
                rest_expenses: expenses
                    - biggest
                        .iter()
                        .filter(|e| e.amount < 0.0)
                        .map(|e| e.amount)
                        .sum::<f64>(),
                biggest,
            }
        })
        .collect();

    // biggest spender first, which is the order a treemap wants to lay out in
    by_category.sort_by(|a, b| a.expenses.total_cmp(&b.expenses));

    Ok(Overviews {
        periods,
        by_category,
    })
}

/// The body of [`AnalysisQuery::history`], outside the resolver so the cache can hold the
/// future without running it.
async fn history_uncached(
    context: &Context,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    step_days: i32,
    accrued: bool,
) -> FieldResult<History> {
    let dates = sample_dates(from, to, step_days)?;
    let rows = simulate(context, &dates, None).await?;

    // Its own statement rather than a column on the simulation: the correction is not per
    // account, and it reads the accrual periods where that one reads recorded balances.
    let accrued_total = if accrued {
        let user_id = require_user(context).await?;
        // Scoped so the connection is back in the pool before the loader below asks for one of
        // its own.
        let adjustments: Vec<AdjustmentRow> = {
            let conn = &mut context.conn().await?;

            diesel::sql_query(ACCRUAL_ADJUSTMENT)
                .bind::<diesel::sql_types::Uuid, _>(user_id)
                .bind::<Array<Timestamptz>, _>(dates.to_vec())
                .load(conn)
                .await?
        };

        let mut totals = vec![0.0; dates.len()];
        for row in &rows {
            // WITH ORDINALITY counts from one
            totals[row.sample_idx as usize - 1] += row.amount;
        }
        for row in &adjustments {
            totals[row.sample_idx as usize - 1] += row.amount;
        }
        totals
    } else {
        vec![]
    };

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
        accrued_total,
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

/// The first day of every period the range touches, plus the first day of the one after it.
///
/// One more boundary than there are periods, because a period needs both its ends: period `i`
/// runs from `starts[i]` to `starts[i + 1]`, and the balance at each boundary is one sample of
/// the same simulation.
///
/// These are also the boundaries the SQL is told to assume: `analysis_movements` files an
/// accrued cost into whole periods, so a range that ended mid-period would have its last one
/// counted whole on one axis and cut off at the day on the other. `analysis_category_expenses`
/// leans on the same snapping to stay grain-free -- see the migration.
fn period_starts(
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    granularity: Granularity,
) -> FieldResult<Vec<DateTime<Utc>>> {
    if to < from {
        return Err(format!("Overview range ends ({to}) before it starts ({from})").into());
    }

    let midnight = |year: i32, month: u32| -> DateTime<Utc> {
        DateTime::from_naive_utc_and_offset(
            chrono::NaiveDate::from_ymd_opt(year, month, 1)
                .expect("the 1st of any month exists")
                .into(),
            Utc,
        )
    };

    let start_of = |d: DateTime<Utc>| -> DateTime<Utc> {
        match granularity {
            Granularity::Month => midnight(d.year(), d.month()),
            Granularity::Year => midnight(d.year(), 1),
        }
    };
    let next = |d: DateTime<Utc>| -> DateTime<Utc> {
        match granularity {
            Granularity::Year => midnight(d.year() + 1, 1),
            Granularity::Month if d.month() == 12 => midnight(d.year() + 1, 1),
            Granularity::Month => midnight(d.year(), d.month() + 1),
        }
    };

    let mut starts = vec![start_of(from)];
    // `to` inside a period means that whole period is wanted, so the boundary after it is the
    // start of the following one
    let last = next(start_of(to));
    while *starts.last().expect("seeded above") < last {
        if starts.len() > MAX_PERIODS {
            return Err(format!(
                "Overview from {from} to {to} covers more than {MAX_PERIODS} periods"
            )
            .into());
        }
        starts.push(next(*starts.last().expect("seeded above")));
    }

    Ok(starts)
}

/// Linear accrual across the accrual period, as [`AnalysisQuery::prepayments`] describes.
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

    // Scoped so the connection is back in the pool before the loader below asks for one of its
    // own. A dataloader batch takes a connection too, and holding one while waiting for another
    // deadlocks as soon as there are as many concurrent requests as the pool has connections:
    // each keeps one, each waits for a second, and the pool never refills.
    let (counts, references) = {
        let conn = &mut context.conn().await?;

        let counts: Vec<ExpenseCountRow> = diesel::sql_query(COUNT_EXPENSES)
            .bind::<diesel::sql_types::Uuid, _>(user_id)
            .bind::<Timestamptz, _>(date)
            .bind::<Nullable<diesel::sql_types::Uuid>, _>(account_id)
            .load(conn)
            .await?;

        let references: Vec<RenderedBalance> = scoped::balances(user_id)
            .filter(
                balances_rendered::id
                    .eq_any(rows.iter().filter_map(|r| r.balance_id).collect::<Vec<_>>()),
            )
            .select(RenderedBalance::as_select())
            .load(conn)
            .await?;

        (counts, references)
    };

    let accounts = context
        .account_loader
        .load_many(references.iter().map(|b| b.account_id).unique().collect())
        .await;

    let mut computed = Vec::with_capacity(references.len());
    for reference in references {
        let row = rows
            .iter()
            .find(|r| r.balance_id == Some(reference.id))
            .expect("every reference balance came out of the computation");
        let account = accounts
            .get(&reference.account_id)
            .ok_or_else(|| format!("Account {} does not exist", reference.account_id))?;

        computed.push(ComputedBalance {
            date,
            amount: row.amount,
            expense_count: counts
                .iter()
                .find(|c| c.account_id == reference.account_id)
                .map_or(0, |c| c.expense_count as i32),
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

    /// A range inside one month is that month, and the boundary after it.
    #[test]
    fn a_month_range_is_two_boundaries() {
        let starts = period_starts(
            t("2026-03-14T09:00:00Z"),
            t("2026-03-20T09:00:00Z"),
            Granularity::Month,
        )
        .unwrap();

        assert_eq!(
            starts,
            vec![t("2026-03-01T00:00:00Z"), t("2026-04-01T00:00:00Z")]
        );
    }

    /// The same range at the coarser grain is one year, not one month of it: both ends snap
    /// outwards to the 1st of a January, which is what lets the SQL file a whole accrued year
    /// into it.
    #[test]
    fn a_year_range_snaps_out_to_januaries() {
        let starts = period_starts(
            t("2026-03-14T09:00:00Z"),
            t("2026-11-20T09:00:00Z"),
            Granularity::Year,
        )
        .unwrap();

        assert_eq!(
            starts,
            vec![t("2026-01-01T00:00:00Z"), t("2027-01-01T00:00:00Z")]
        );
    }

    /// A period `to` falls inside is wanted whole, so a range crossing New Year is two years
    /// rather than the fortnight it names.
    #[test]
    fn a_year_range_crossing_new_year_holds_both_of_them() {
        let starts = period_starts(
            t("2025-12-28T00:00:00Z"),
            t("2026-01-04T00:00:00Z"),
            Granularity::Year,
        )
        .unwrap();

        assert_eq!(
            starts,
            vec![
                t("2025-01-01T00:00:00Z"),
                t("2026-01-01T00:00:00Z"),
                t("2027-01-01T00:00:00Z")
            ]
        );
    }

    /// December has to roll the year over, and it is the one month that does.
    #[test]
    fn a_month_range_rolls_the_year_over() {
        let starts = period_starts(
            t("2026-12-05T00:00:00Z"),
            t("2026-12-06T00:00:00Z"),
            Granularity::Month,
        )
        .unwrap();

        assert_eq!(
            starts,
            vec![t("2026-12-01T00:00:00Z"), t("2027-01-01T00:00:00Z")]
        );
    }

    /// The grain decides how much a range costs to answer: the decade that is 121 boundaries
    /// of months is 11 of years, which is the point of having it.
    #[test]
    fn the_grain_decides_how_many_boundaries_a_decade_is() {
        let (from, to) = (t("2016-01-01T00:00:00Z"), t("2025-12-31T00:00:00Z"));

        assert_eq!(
            period_starts(from, to, Granularity::Month).unwrap().len(),
            121
        );
        assert_eq!(
            period_starts(from, to, Granularity::Year).unwrap().len(),
            11
        );
    }

    #[test]
    fn a_backwards_range_is_refused_at_either_grain() {
        let (from, to) = (t("2026-06-01T00:00:00Z"), t("2026-01-01T00:00:00Z"));

        assert!(period_starts(from, to, Granularity::Month).is_err());
        assert!(period_starts(from, to, Granularity::Year).is_err());
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
