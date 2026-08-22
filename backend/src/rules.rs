//! Turning rules into expenses.
//!
//! The interesting parts here are pure so they can be tested without a database: which
//! instants a periodic rule is due for, what the generated expense looks like, and which
//! automation rule a delivered transaction matches.

use std::error::Error;

use chrono::{DateTime, Datelike, TimeDelta, Utc};
use diesel::prelude::*;
use diesel::sql_types::BigInt;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use regex::Regex;
use uuid::Uuid;

use crate::accrual;
use crate::models::{
    AutomationRule, Expense, ExpenseChangelogEntry, ExpenseDelivery, ExpenseTransaction, Json,
    PeriodicRule, SeenDelivery,
};
use crate::queries;
use crate::schema::{automation_rules, expense_changes, periodic_rules, seen_deliveries};

/// The instants a rule is due for: stepping from the template's booking start by the rule's
/// period, keeping those after `last_creation` and no later than `until`.
///
/// Anchoring on the template rather than on `last_creation` keeps occurrences on the day of
/// the month the user picked, however long the rule has been dormant.
pub fn occurrences(
    rule: &PeriodicRule,
    template: &Expense,
    until: DateTime<Utc>,
) -> Vec<DateTime<Utc>> {
    let mut res = Vec::new();
    let mut t = template.accrual_start;

    // a template dated far in the past would otherwise iterate for a long time before
    // reaching the window, but this is bounded by the period and stays cheap in practice
    while t <= until {
        if t > rule.last_creation {
            res.push(t);
        }

        let next = rule.period.next(t);
        if next <= t {
            break; // defensive: a period that does not advance would loop forever
        }
        t = next;
    }

    res
}

/// Builds the expense a rule produces for one occurrence.
///
/// The whole booking window and every transaction date shift by the same delta, so a
/// template spanning several days keeps its shape.
pub fn instantiate(rule: &PeriodicRule, template: &Expense, at: DateTime<Utc>) -> Expense {
    let shift = at - template.accrual_start;

    let mut e = template.clone();
    e.id = Uuid::now_v7();
    e.is_template = false;
    e.accrual_start = at;
    e.accrual_end = template.accrual_end + shift;
    e.created_at = at;
    e.updated_at = at;

    for t in e.transactions.iter_mut() {
        t.date += shift;
    }

    if let Some(title) = &rule.title_replacement {
        e.title = substitute_date(title, at);
    }

    e
}

/// `%d`, `%m` and `%y` in a title replacement, as moneydb defined them: no zero padding.
fn substitute_date(title: &str, at: DateTime<Utc>) -> String {
    title
        .replace("%d", &at.day().to_string())
        .replace("%m", &at.month().to_string())
        .replace("%y", &at.year().to_string())
}

/// Records that a rule, rather than a person, created this expense.
pub fn changelog_entry(expense: &Expense, user_id: Uuid, tool: &str) -> ExpenseChangelogEntry {
    ExpenseChangelogEntry {
        id: Uuid::now_v7(),
        target_id: expense.id,
        user_id: Some(user_id),
        date: expense.created_at,
        tool: Some(tool.to_owned()),
        automatic: true,
        confidence: 1.0,
        old_state: None,
        new_state: Some(Json(expense.clone())),
    }
}

/// Whether a delivered transaction satisfies every filter a rule sets. Absent filters match
/// anything, so a rule with no filters at all matches everything.
pub fn matches(rule: &AutomationRule, delivery: &ExpenseDelivery) -> bool {
    let regex_matches = |pattern: &Option<String>, haystack: &str| match pattern {
        None => true,
        // an unparseable pattern must not match everything, so treat it as no match
        Some(p) => Regex::new(p).map(|r| r.is_match(haystack)).unwrap_or(false),
    };

    regex_matches(&rule.regex_time, &delivery.formatted_date())
        && regex_matches(&rule.regex_statement, &delivery.statement)
        && rule.filter_amount.is_none_or(|a| a == delivery.amount)
        && rule
            .filter_account_id
            .is_none_or(|a| a == delivery.account_id)
}

/// The rule that should handle a delivery: highest priority among those that match, ties
/// broken by title so the outcome does not depend on database order.
pub fn best_match<'a>(
    rules: &'a [AutomationRule],
    delivery: &ExpenseDelivery,
) -> Option<&'a AutomationRule> {
    rules
        .iter()
        .filter(|r| matches(r, delivery))
        .max_by(|a, b| a.priority.cmp(&b.priority).then(b.title.cmp(&a.title)))
}

/// Builds the expense an automation rule produces, imposing the delivered transaction's
/// data onto the template's booking.
pub fn impose(
    template: &Expense,
    delivery: &ExpenseDelivery,
    now: DateTime<Utc>,
) -> Result<Expense, Box<dyn Error>> {
    let mut e = template.clone();
    e.id = Uuid::now_v7();
    e.is_template = false;
    // the delivery is a bank statement line, so it is worth a human look before it counts
    e.is_unchecked = true;
    e.created_at = now;
    e.updated_at = now;

    // The delivery takes the place of exactly one line, and the rest of the template -- the
    // counter-bookings and the shares -- is what it gets wrapped in.
    let paid = template_booking(template);
    if let Some(idx) = booking_of(template, delivery.account_id) {
        e.transactions.remove(idx);
    }

    let (start, end) = period(template, paid, delivery);
    e.accrual_start = start;
    e.accrual_end = end;

    // The rest of the template hangs off its booking, not off its period: a counter-booking a
    // day after the payment is a day after this payment too.
    let shift = delivery.date - paid;
    for t in e.transactions.iter_mut() {
        t.date += shift;
    }

    // Whether anything takes a share of this booking decides how it is written: beside shares it
    // is the whole of the reference amount, so the money is written down once and a correction
    // moves all of it together. Alone -- or beside a fixed share, which is somebody's 9,99 EUR
    // of the phone bill however big the bill -- it is simply what the bank said.
    let shared = e.transactions.iter().any(|t| t.fraction.is_some());
    e.reference_amount = shared.then_some(delivery.amount);

    // first, where the booking of an expense belongs and where the template kept its blank one
    e.transactions.insert(
        0,
        ExpenseTransaction {
            account_id: delivery.account_id,
            date: delivery.date,
            booking_date: delivery.booking_date,
            amount: (!shared).then_some(delivery.amount),
            fraction: shared.then_some(1.0),
            comments: String::new(),
            statement: delivery.statement.clone(),
        },
    );

    Ok(e)
}

/// The period a delivered payment is *for*.
///
/// What the bank prints beats what the template implies: a broadcaster or an insurer names the
/// period on the line, and no heuristic improves on being told. Failing that, the template says
/// how a payment sits inside its own period and the payment is placed the same way — see
/// [`accrual::Shape`].
fn period(
    template: &Expense,
    template_paid: DateTime<Utc>,
    delivery: &ExpenseDelivery,
) -> (DateTime<Utc>, DateTime<Utc>) {
    accrual::statement_period(&delivery.statement, delivery.date).unwrap_or_else(|| {
        accrual::Shape::of(template.accrual_start, template.accrual_end, template_paid)
            .period(delivery.date)
    })
}

/// The date a template says its own payment fell on, which is what [`accrual::Shape`] measures
/// the phase against — how far into the period it pays for a payment lands.
///
/// It is the blank booking's: the transaction of exactly zero that an automation template keeps
/// in place of a payment. Zero because there is no amount to state before a delivery brings
/// one, and kept because an expense with no transactions belongs to no user at all —
/// `expense_users` derives ownership from the accounts its transactions touch, so a template
/// without one would render in no view and could not be opened.
///
/// A template that has lost its blank booking falls back on the start of its own period, which
/// is a phase of zero: paid on the first day of what it pays for.
pub fn template_booking(template: &Expense) -> DateTime<Utc> {
    template
        .transactions
        .iter()
        .find(|t| t.amount == Some(0))
        .map_or(template.accrual_start, |t| t.date)
}

/// The line a delivery takes the place of.
///
/// An imported template names it outright: `blank_booking` left one transaction of exactly zero
/// where the payment goes, so nothing depends on order or on which account it happens to name.
///
/// A template made by hand has no such line — you duplicate an expense and tick *Vorlage*, and
/// it keeps the booking it was duplicated from. Left in place that booking is kept *and* the
/// delivery added beside it, which books the money twice: silently, since both spellings pass
/// `validate`, and worst for a split, where the editor writes the booking as `fraction = 1` and
/// two of those against one reference amount come to 200 % of the bill. So failing the
/// placeholder it is the line on the account the money arrived on, and failing that the first,
/// which is where a booking sits.
fn booking_of(template: &Expense, delivered_to: Uuid) -> Option<usize> {
    template
        .transactions
        .iter()
        .position(|t| t.amount == Some(0))
        .or_else(|| {
            template
                .transactions
                .iter()
                .position(|t| t.account_id == delivered_to)
        })
        .or_else(|| (!template.transactions.is_empty()).then_some(0))
}

/// What a transaction is worth, whichever way it is written down: the amount it carries, or
/// its share of what the expense as a whole came to.
pub fn resolved_amount(expense: &Expense, t: &ExpenseTransaction) -> Option<i32> {
    match (t.amount, t.fraction) {
        (Some(amount), _) => Some(amount),
        (None, Some(f)) => {
            Some((f * f64::from(expense.reference_amount?)).round_ties_even() as i32)
        }
        (None, None) => None,
    }
}

/// The bank line an expense came from, put back together so the rules can be run over it
/// again. `None` for anything typed in by hand, which has nothing for a rule to match on.
///
/// The transaction carrying a statement is the one the bank delivered; the others are the
/// shares and counter-bookings the template added around it. Where an expense holds several --
/// a split paid from two banks -- the first is taken, which is the one `impose` wrote.
///
/// A shared expense books that line as the whole of itself rather than as a figure of its own,
/// so what it is worth has to be resolved rather than read off.
pub fn delivery_of(expense: &Expense) -> Option<ExpenseDelivery> {
    deliveries_of(expense).next()
}

/// Every bank line an expense holds, in transaction order.
///
/// `delivery_of` wants the one a rule was built from and stops at the first; this is for
/// asking what the bank has already handed over, where all of them count -- a purchase
/// and the shop's refund arrive as two lines and end up in one expense.
///
/// A line whose worth cannot be resolved is skipped rather than ending the walk, or an expense
/// would report having no bank transaction at all while a perfectly good one sits below it.
pub fn deliveries_of(expense: &Expense) -> impl Iterator<Item = ExpenseDelivery> + '_ {
    expense.transactions.iter().filter_map(|t| {
        if t.statement.trim().is_empty() {
            return None;
        }
        Some(ExpenseDelivery {
            account_id: t.account_id,
            date: t.date,
            booking_date: t.booking_date,
            amount: resolved_amount(expense, t)?,
            statement: t.statement.clone(),
        })
    })
}

/// How far apart two otherwise identical bank lines have to be to count as separate
/// transactions. Statements carry no stable per-line id, so this is the only thing standing
/// between a re-upload and a duplicated expense.
///
/// Inherited from moneydb. It does mean a genuine second identical payment on the same day
/// is treated as a duplicate, which is the safer way round to be wrong.
pub const DELIVERY_DEDUP_WINDOW: TimeDelta = TimeDelta::hours(24);

/// How far back a delivery is remembered. Beyond this nothing is compared, so it is also as
/// far back as there is any point in seeding the table from an import.
pub const DELIVERY_MEMORY: TimeDelta = TimeDelta::days(400);

/// Whether this delivery has already been processed: the same amount on the same account,
/// close enough in time. The statement text is deliberately not compared — banks reword it
/// between a provisional and a final booking of the same payment.
pub fn is_duplicate(seen: &[SeenDelivery], delivery: &ExpenseDelivery) -> bool {
    seen.iter().any(|s| {
        s.account_id == delivery.account_id
            && s.amount == delivery.amount
            && (s.date - delivery.date).abs() < DELIVERY_DEDUP_WINDOW
    })
}

/// How far ahead of `now` periodic expenses are generated, so a rule due later today is
/// already visible.
pub const PERIODIC_ADVANCE: TimeDelta = TimeDelta::hours(12);

/// What happened to a single delivered transaction.
#[derive(Debug, Clone)]
pub enum DeliveryOutcome {
    /// an expense was created from the named rule
    Created { expense_id: Uuid, rule_id: Uuid },
    /// already processed, most likely an overlapping re-upload
    Duplicate,
    /// nothing matched, so the line is recorded but no expense is created
    Unmatched,
    /// a rule matched but its template could not absorb the delivery
    Failed { message: String },
}

/// Processes delivered bank transactions for one user: deduplicate, match against that
/// user's automation rules, and create the resulting expenses.
///
/// Every delivery is recorded as seen, matched or not, so that re-uploading the same
/// statement does not keep re-evaluating the same lines.
pub async fn deliver_expenses(
    conn: &mut AsyncPgConnection,
    user_id: Uuid,
    deliveries: &[ExpenseDelivery],
    now: DateTime<Utc>,
) -> Result<Vec<DeliveryOutcome>, Box<dyn Error + Send + Sync>> {
    let rules = automation_rules::table
        .filter(automation_rules::user_id.eq(user_id))
        .select(AutomationRule::as_select())
        .load(conn)
        .await?;

    // one query for the whole batch rather than per delivery; a user's history on the
    // accounts being delivered to is small enough to filter in memory
    let account_ids: Vec<_> = deliveries.iter().map(|d| d.account_id).collect();
    let mut seen = seen_deliveries::table
        .filter(seen_deliveries::user_id.eq(user_id))
        .filter(seen_deliveries::account_id.eq_any(&account_ids))
        .filter(seen_deliveries::date.ge(now - DELIVERY_MEMORY))
        .select(SeenDelivery::as_select())
        .load(conn)
        .await?;

    let mut outcomes = Vec::with_capacity(deliveries.len());

    for delivery in deliveries {
        // deliveries within one batch can duplicate each other too, so `seen` grows as we go
        if is_duplicate(&seen, delivery) {
            outcomes.push(DeliveryOutcome::Duplicate);
            continue;
        }

        let mut record = SeenDelivery {
            id: Uuid::now_v7(),
            user_id,
            account_id: delivery.account_id,
            date: delivery.date,
            amount: delivery.amount,
            statement: delivery.statement.clone(),
            expense_id: None,
            created_at: now,
        };

        let outcome = match best_match(&rules, delivery) {
            None => DeliveryOutcome::Unmatched,
            Some(rule) => match queries::load_expense(conn, rule.template_id).await? {
                None => DeliveryOutcome::Failed {
                    message: format!("rule {} has no template expense", rule.title),
                },
                // stringified eagerly: a boxed dyn Error is not Send, and holding one
                // across the awaits below would make this future unspawnable
                Some(template) => match impose(&template, delivery, now)
                    .and_then(|e| e.validate().map(|_| e))
                    .map_err(|e| e.to_string())
                {
                    Err(message) => DeliveryOutcome::Failed { message },
                    Ok(expense) => {
                        let expense_id = expense.id;
                        let rule_id = rule.id;

                        conn.transaction::<_, diesel::result::Error, _>(async |conn| {
                            queries::insert_expense(conn, &expense).await?;

                            diesel::insert_into(expense_changes::table)
                                .values(changelog_entry(&expense, user_id, "moneta-automation"))
                                .execute(conn)
                                .await?;

                            diesel::update(automation_rules::table.find(rule_id))
                                .set((
                                    automation_rules::last_delivery.eq(now),
                                    automation_rules::updated_at.eq(now),
                                ))
                                .execute(conn)
                                .await?;

                            Ok(())
                        })
                        .await?;

                        record.expense_id = Some(expense_id);
                        DeliveryOutcome::Created {
                            expense_id,
                            rule_id,
                        }
                    }
                },
            },
        };

        // a failed delivery is deliberately not recorded, so fixing the rule and
        // re-uploading picks it up instead of silently swallowing it forever
        if !matches!(outcome, DeliveryOutcome::Failed { .. }) {
            diesel::insert_into(seen_deliveries::table)
                .values(&record)
                .execute(conn)
                .await?;
            seen.push(record);
        }

        outcomes.push(outcome);
    }

    Ok(outcomes)
}

/// An arbitrary but stable key for `pg_advisory_xact_lock`. Advisory locks share one
/// namespace per database, so the only thing that matters is that nothing else picks it.
const PERIODIC_LOCK: i64 = 0x6d6f6e_65746131;

/// Runs every periodic rule that is due, writing the generated expenses and their changelog
/// entries, then advancing each rule's `last_creation`.
///
/// `user_id` restricts the run to one user's rules; `None` runs all of them, which is what the
/// scheduled job does.
///
/// Idempotent: running it twice in a row generates nothing the second time.
pub async fn generate_periodic_expenses(
    conn: &mut AsyncPgConnection,
    user_id: Option<Uuid>,
    now: DateTime<Utc>,
) -> Result<usize, Box<dyn Error + Send + Sync>> {
    let until = now + PERIODIC_ADVANCE;

    // The whole run is one transaction, reads included. `last_creation` is what decides which
    // occurrences are due, so two runs that both read it before either commits would generate
    // the same expenses twice -- and the advisory lock is taken before the first read for the
    // same reason. It is released when the transaction ends, whichever way it ends.
    //
    // The lock is global rather than per user even when the run is scoped, so that a user
    // triggering their own generation still excludes the scheduled run over everybody's rules.
    let count = conn
        .transaction::<_, diesel::result::Error, _>(async |conn| {

            diesel::sql_query("SELECT pg_advisory_xact_lock($1)")
                .bind::<BigInt, _>(PERIODIC_LOCK)
                .execute(conn)
                .await?;

            let mut query = periodic_rules::table.into_boxed();
            if let Some(user_id) = user_id {
                query = query.filter(periodic_rules::user_id.eq(user_id));
            }
            let rules = query.select(PeriodicRule::as_select()).load(conn).await?;

            let mut generated = Vec::new();
            let mut advanced = Vec::new();

            for rule in &rules {
                let Some(template) = queries::load_expense(conn, rule.template_id).await? else {
                    warn!(
                        "periodic rule {} ({}) points at a template that no longer exists, skipping",
                        rule.id, rule.title
                    );
                    continue;
                };

                let times = occurrences(rule, &template, until);
                if times.is_empty() {
                    continue;
                }

                let mut written = None;
                for t in &times {
                    let expense = instantiate(rule, &template, *t);
                    if let Err(e) = expense.validate() {
                        // Stop rather than skip on. `last_creation` may only move past
                        // occurrences that were actually written: advancing past a bad one
                        // loses that month for good while the rule goes on looking healthy.
                        // Repairing the template and running again resumes here.
                        warn!(
                            "periodic rule {} ({}) would produce an invalid expense for {}, \
                             stopping there: {}",
                            rule.id,
                            rule.title,
                            t.date_naive(),
                            e
                        );
                        break;
                    }
                    generated.push((expense, rule.user_id));
                    written = Some(*t);
                }

                if let Some(last) = written {
                    advanced.push((rule.id, last));
                }
            }

            for (expense, owner) in &generated {
                queries::insert_expense(conn, expense).await?;
                diesel::insert_into(expense_changes::table)
                    .values(changelog_entry(expense, *owner, "moneta-periodic"))
                    .execute(conn)
                    .await?;
            }

            for (id, last) in &advanced {
                diesel::update(periodic_rules::table.find(id))
                    .set((
                        periodic_rules::last_creation.eq(last),
                        periodic_rules::updated_at.eq(now),
                    ))
                    .execute(conn)
                    .await?;
            }

            Ok(generated.len())
        })
        .await?;

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ExpenseTransaction, PeriodicInterval};
    use chrono::TimeZone;

    fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 12, 0, 0).unwrap()
    }

    fn template(transactions: Vec<ExpenseTransaction>) -> Expense {
        Expense {
            id: Uuid::now_v7(),
            title: "Rundfunkgebühr".into(),
            store: String::new(),
            description: String::new(),
            comments: String::new(),
            reference_amount: None,
            accrual_start: at(2025, 1, 15),
            accrual_end: at(2025, 1, 15),
            is_template: true,
            is_tax_relevant: false,
            is_unchecked: false,
            is_incomplete: false,
            is_extraordinary: false,
            receipts: vec![],
            categories: vec![],
            transactions,
            positions: vec![],
            created_at: at(2025, 1, 15),
            updated_at: at(2025, 1, 15),
        }
    }

    fn transaction(account_id: Uuid, amount: Option<i32>) -> ExpenseTransaction {
        ExpenseTransaction {
            account_id,
            date: at(2025, 1, 15),
            booking_date: None,
            amount,
            fraction: None,
            comments: String::new(),
            statement: String::new(),
        }
    }

    #[test]
    fn the_delivery_behind_an_expense_is_the_transaction_that_carries_a_statement() {
        let bank = Uuid::now_v7();
        let cash = Uuid::now_v7();

        // a share of the expense, and no bank line of its own
        let mut share = transaction(cash, None);
        share.fraction = Some(0.5);

        let mut delivered = transaction(bank, Some(-1849));
        delivered.statement = "REWE SAGT HALLO".into();

        let expense = template(vec![share, delivered]);
        let found = delivery_of(&expense).expect("the bank line is in there");

        assert_eq!(found.account_id, bank);
        assert_eq!(found.amount, -1849);
        assert_eq!(found.statement, "REWE SAGT HALLO");
    }

    #[test]
    fn an_expense_typed_in_by_hand_has_no_delivery_to_sort_by() {
        let expense = template(vec![transaction(Uuid::now_v7(), Some(-1849))]);
        assert!(delivery_of(&expense).is_none());
    }

    /// `delivery_of` wants the line a rule was built from and stops at the first; asking what
    /// the bank has handed over needs all of them, and a refund is a second line on the same
    /// expense
    #[test]
    fn an_expense_can_hold_more_than_one_bank_line() {
        let bank = Uuid::now_v7();

        let mut bill = transaction(bank, Some(-12000));
        bill.statement = "MUSTERHANDEL".into();
        let mut refund = transaction(bank, Some(4000));
        refund.statement = "MUSTERHANDEL GUTSCHRIFT".into();

        let expense = template(vec![bill, refund]);

        assert_eq!(
            deliveries_of(&expense)
                .map(|d| d.amount)
                .collect::<Vec<_>>(),
            vec![-12000, 4000]
        );
        assert_eq!(delivery_of(&expense).map(|d| d.amount), Some(-12000));
    }

    fn periodic(period: PeriodicInterval, last_creation: DateTime<Utc>) -> PeriodicRule {
        PeriodicRule {
            id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            title: "Rundfunkgebühr".into(),
            template_id: Uuid::now_v7(),
            period,
            title_replacement: None,
            last_creation,
            created_at: at(2025, 1, 1),
            updated_at: at(2025, 1, 1),
        }
    }

    fn automation(title: &str, priority: i32) -> AutomationRule {
        AutomationRule {
            id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            title: title.into(),
            template_id: Uuid::now_v7(),
            priority,
            regex_time: None,
            regex_statement: None,
            filter_amount: None,
            filter_account_id: None,
            last_delivery: None,
            created_at: at(2025, 1, 1),
            updated_at: at(2025, 1, 1),
        }
    }

    fn delivery(account_id: Uuid, amount: i32, statement: &str) -> ExpenseDelivery {
        ExpenseDelivery {
            account_id,
            date: at(2025, 3, 4),
            booking_date: None,
            amount,
            statement: statement.into(),
        }
    }

    #[test]
    fn monthly_rule_is_due_once_per_month_since_last_creation() {
        let rule = periodic(PeriodicInterval::Monthly, at(2025, 3, 1));
        let ts = occurrences(&rule, &template(vec![]), at(2025, 6, 1));

        // march..may: june's occurrence is on the 15th, past the cutoff
        assert_eq!(ts, vec![at(2025, 3, 15), at(2025, 4, 15), at(2025, 5, 15)]);
    }

    #[test]
    fn nothing_is_due_when_the_rule_is_up_to_date() {
        let rule = periodic(PeriodicInterval::Monthly, at(2025, 6, 1));
        assert!(occurrences(&rule, &template(vec![]), at(2025, 6, 10)).is_empty());
    }

    /// occurrences follow the template's day of month, not the last run
    #[test]
    fn occurrences_stay_anchored_to_the_template() {
        let rule = periodic(PeriodicInterval::Monthly, at(2025, 4, 3));
        let ts = occurrences(&rule, &template(vec![]), at(2025, 7, 1));

        assert!(ts.iter().all(|t| t.day() == 15));
    }

    #[test]
    fn weekly_rule_steps_by_seven_days() {
        let rule = periodic(PeriodicInterval::Weekly, at(2025, 1, 20));
        let ts = occurrences(&rule, &template(vec![]), at(2025, 2, 10));

        assert_eq!(ts, vec![at(2025, 1, 22), at(2025, 1, 29), at(2025, 2, 5)]);
    }

    #[test]
    fn instantiated_expense_is_not_itself_a_template() {
        let rule = periodic(PeriodicInterval::Monthly, at(2025, 3, 1));
        let tpl = template(vec![transaction(Uuid::now_v7(), Some(-1836))]);
        let e = instantiate(&rule, &tpl, at(2025, 4, 15));

        assert!(!e.is_template);
        assert_ne!(e.id, tpl.id);
        assert_eq!(e.accrual_start, at(2025, 4, 15));
    }

    /// a template booked across several days keeps that span, and its transactions move with it
    #[test]
    fn instantiation_shifts_the_whole_booking_window() {
        let rule = periodic(PeriodicInterval::Monthly, at(2025, 3, 1));
        let mut tpl = template(vec![transaction(Uuid::now_v7(), Some(-500))]);
        tpl.accrual_end = at(2025, 1, 18);

        let e = instantiate(&rule, &tpl, at(2025, 4, 15));

        assert_eq!(e.accrual_start, at(2025, 4, 15));
        assert_eq!(e.accrual_end, at(2025, 4, 18));
        assert_eq!(e.transactions[0].date, at(2025, 4, 15));
    }

    #[test]
    fn title_replacement_substitutes_the_occurrence_date() {
        let mut rule = periodic(PeriodicInterval::Monthly, at(2025, 3, 1));
        rule.title_replacement = Some("Ausgleichszahlung %m/%y".into());

        let e = instantiate(&rule, &template(vec![]), at(2025, 4, 15));
        assert_eq!(e.title, "Ausgleichszahlung 4/2025");
    }

    #[test]
    fn a_rule_without_filters_matches_anything() {
        let rule = automation("catch all", 1);
        assert!(matches(&rule, &delivery(Uuid::now_v7(), -100, "whatever")));
    }

    #[test]
    fn statement_regex_must_match() {
        let mut rule = automation("Abfall", 1);
        rule.regex_statement = Some("Muster Stadtreinigung".into());

        assert!(matches(
            &rule,
            &delivery(Uuid::now_v7(), -100, "LASTSCHRIFT Muster Stadtreinigung AG")
        ));
        assert!(!matches(
            &rule,
            &delivery(Uuid::now_v7(), -100, "REWE SAGT DANKE")
        ));
    }

    #[test]
    fn amount_and_account_filters_must_match_exactly() {
        let account = Uuid::now_v7();
        let mut rule = automation("exact", 1);
        rule.filter_amount = Some(-5001);
        rule.filter_account_id = Some(account);

        assert!(matches(&rule, &delivery(account, -5001, "x")));
        assert!(!matches(&rule, &delivery(account, -5000, "x")));
        assert!(!matches(&rule, &delivery(Uuid::now_v7(), -5001, "x")));
    }

    #[test]
    fn time_regex_matches_the_formatted_delivery_date() {
        let mut rule = automation("every fourth", 1);
        rule.regex_time = Some(r"^04\.03\.2025".into());

        assert!(matches(&rule, &delivery(Uuid::now_v7(), -100, "x")));
    }

    /// a rule nobody can parse must not silently become a catch-all
    #[test]
    fn an_invalid_regex_matches_nothing() {
        let mut rule = automation("broken", 1);
        rule.regex_statement = Some("([unclosed".into());

        assert!(!matches(&rule, &delivery(Uuid::now_v7(), -100, "anything")));
    }

    #[test]
    fn the_highest_priority_matching_rule_wins() {
        let account = Uuid::now_v7();
        let mut specific = automation("specific", 10);
        specific.regex_statement = Some("REWE".into());
        let general = automation("general", 1);

        let rules = vec![general, specific];
        let picked = best_match(&rules, &delivery(account, -100, "REWE SAGT DANKE")).unwrap();

        assert_eq!(picked.title, "specific");
    }

    #[test]
    fn no_match_yields_nothing() {
        let mut rule = automation("nope", 1);
        rule.regex_statement = Some("ALDI".into());

        assert!(
            best_match(
                std::slice::from_ref(&rule),
                &delivery(Uuid::now_v7(), -100, "REWE")
            )
            .is_none()
        );
    }

    /// The delivered amount has to land somewhere. A template that books nothing against the
    /// delivered account would otherwise yield an expense carrying the template's amounts,
    /// which is wrong money written silently — the same failure the periodic amount
    /// replacement had.
    fn seen(account_id: Uuid, amount: i32, date: DateTime<Utc>) -> SeenDelivery {
        SeenDelivery {
            id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            account_id,
            date,
            amount,
            statement: "whatever".into(),
            expense_id: None,
            created_at: date,
        }
    }

    #[test]
    fn an_identical_line_on_the_same_day_is_a_duplicate() {
        let account = Uuid::now_v7();
        let history = vec![seen(account, -4237, at(2025, 3, 4))];

        assert!(is_duplicate(&history, &delivery(account, -4237, "REWE")));
    }

    /// banks reword the statement between a provisional and a final booking of the same
    /// payment, so the text must not decide identity
    #[test]
    fn a_reworded_statement_is_still_a_duplicate() {
        let account = Uuid::now_v7();
        let history = vec![seen(account, -4237, at(2025, 3, 4))];

        let mut d = delivery(account, -4237, "KARTENZAHLUNG REWE SAGT DANKE 04.03.");
        d.date = at(2025, 3, 4);

        assert!(is_duplicate(&history, &d));
    }

    #[test]
    fn a_different_amount_or_account_is_not_a_duplicate() {
        let account = Uuid::now_v7();
        let history = vec![seen(account, -4237, at(2025, 3, 4))];

        assert!(!is_duplicate(&history, &delivery(account, -4238, "REWE")));
        assert!(!is_duplicate(
            &history,
            &delivery(Uuid::now_v7(), -4237, "REWE")
        ));
    }

    /// the same standing order next month is a real second payment
    #[test]
    fn the_same_amount_outside_the_window_is_not_a_duplicate() {
        let account = Uuid::now_v7();
        let history = vec![seen(account, -4237, at(2025, 2, 4))];

        assert!(!is_duplicate(&history, &delivery(account, -4237, "REWE")));
    }

    /// a bank moving a booking by a few hours must not produce a second expense
    #[test]
    fn a_delivery_shifted_within_the_window_is_a_duplicate() {
        let account = Uuid::now_v7();
        let history = vec![seen(account, -4237, at(2025, 3, 4) - TimeDelta::hours(20))];

        assert!(is_duplicate(&history, &delivery(account, -4237, "REWE")));
    }

    #[test]
    fn an_empty_history_never_matches() {
        assert!(!is_duplicate(&[], &delivery(Uuid::now_v7(), -100, "x")));
    }

    /// The delivery says which account the money moved on, so a template written against a
    /// different one still books it. One REWE rule serves the card payment and the cash
    /// purchase; before this, whichever account the template did not name simply failed.
    #[test]
    fn a_delivery_is_booked_on_its_own_account() {
        let cash = Uuid::now_v7();
        let card = Uuid::now_v7();
        let tpl = template(vec![transaction(cash, Some(0))]);

        let e = impose(&tpl, &delivery(card, -4237, "REWE"), at(2025, 3, 5)).unwrap();

        assert_eq!(e.transactions.len(), 1);
        assert_eq!(e.transactions[0].account_id, card);
        assert_eq!(e.transactions[0].amount, Some(-4237));
    }

    /// The placeholder is recognised by its zero, not by where it sits, so editing a template
    /// afterwards cannot silently make a rule book a different line.
    #[test]
    fn the_blank_booking_is_found_wherever_it_sits() {
        let giro = Uuid::now_v7();
        let mut tpl = template(vec![
            transaction(Uuid::now_v7(), Some(999)),
            transaction(giro, Some(0)),
        ]);
        tpl.transactions[1].date = at(2025, 1, 9);

        assert_eq!(template_booking(&tpl), at(2025, 1, 9));

        let e = impose(&tpl, &delivery(giro, -4237, "x"), at(2025, 3, 5)).unwrap();

        // the delivery goes first and the placeholder is gone; the fixed share survives
        assert_eq!(e.transactions.len(), 2);
        assert_eq!(e.transactions[0].amount, Some(-4237));
        assert_eq!(e.transactions[1].amount, Some(999));
    }

    /// A template made the documented way — duplicate an expense, tick *Vorlage* — keeps the
    /// booking it was duplicated from, and there is no placeholder to recognise. The delivery
    /// has to take that booking's place rather than land beside it.
    #[test]
    fn a_hand_made_template_does_not_book_its_own_amount_as_well() {
        let account = Uuid::now_v7();
        let tpl = template(vec![transaction(account, Some(-1750))]);

        let e = impose(&tpl, &delivery(account, -4237, "x"), at(2025, 3, 5)).unwrap();

        assert_eq!(e.transactions.len(), 1, "{:?}", e.transactions);
        assert_eq!(e.transactions[0].amount, Some(-4237));
    }

    /// The same for a split, where it is worse: the editor writes a booking as `fraction = 1`,
    /// so leaving it in place gives two whole shares of one reference amount — 200 % of the
    /// bill, and `validate` has nothing to object to.
    #[test]
    fn a_hand_made_split_template_does_not_book_itself_twice() {
        let account = Uuid::now_v7();
        let mut tpl = template(vec![
            transaction(account, None),
            transaction(Uuid::now_v7(), None),
        ]);
        tpl.transactions[0].fraction = Some(1.0);
        tpl.transactions[1].fraction = Some(-0.5);
        tpl.reference_amount = Some(-1750);

        let e = impose(&tpl, &delivery(account, -4237, "x"), at(2025, 3, 5)).unwrap();

        assert_eq!(e.reference_amount, Some(-4237));
        assert_eq!(
            e.transactions
                .iter()
                .filter(|t| t.fraction == Some(1.0))
                .count(),
            1,
            "one whole, not two: {:?}",
            e.transactions
        );
        let total: i32 = e
            .transactions
            .iter()
            .filter_map(|t| resolved_amount(&e, t))
            .sum();
        assert_eq!(total, -2119, "the payment less the half that is owed back");
    }

    /// nothing to place it against is not a failure: the delivery is simply the whole of the
    /// expense, which is what a rule with a bare template produces
    #[test]
    fn a_template_with_no_transactions_still_books_the_delivery() {
        let account = Uuid::now_v7();

        let e = impose(
            &template(vec![]),
            &delivery(account, -4237, "x"),
            at(2025, 3, 5),
        )
        .unwrap();

        assert_eq!(e.transactions.len(), 1);
        assert_eq!(e.transactions[0].account_id, account);
        assert_eq!(e.transactions[0].amount, Some(-4237));
    }

    /// the counter-bookings are shares of the delivered amount, so they must stay fractional
    /// and follow the new reference amount rather than keeping the template's numbers
    #[test]
    fn imposing_a_delivery_rescales_the_shares() {
        let account = Uuid::now_v7();
        let mut tpl = template(vec![
            transaction(account, Some(0)),
            transaction(Uuid::now_v7(), None),
        ]);
        tpl.transactions[1].fraction = Some(-0.5);
        tpl.reference_amount = Some(0);

        let e = impose(&tpl, &delivery(account, -4237, "REWE"), at(2025, 3, 5)).unwrap();

        // beside a share the booking is the whole of the reference amount, so the money is
        // written down once and a correction moves all of it at the same time
        assert_eq!(e.transactions[0].fraction, Some(1.0));
        assert_eq!(e.transactions[0].amount, None);
        assert_eq!(e.reference_amount, Some(-4237));
        assert_eq!(e.transactions[1].fraction, Some(-0.5));
        assert!(e.validate().is_ok());
        assert_eq!(delivery_of(&e).map(|d| d.amount), Some(-4237));
    }

    /// with nothing taking a share of it, the booking is simply what the bank said and there is
    /// no reference amount for anything to be a fraction of
    #[test]
    fn imposing_a_delivery_nobody_shares_leaves_a_fixed_booking() {
        let account = Uuid::now_v7();
        let tpl = template(vec![
            transaction(account, Some(0)),
            transaction(Uuid::now_v7(), Some(999)),
        ]);

        let e = impose(&tpl, &delivery(account, -4237, "x"), at(2025, 3, 5)).unwrap();

        assert_eq!(e.transactions[0].amount, Some(-4237));
        assert_eq!(e.transactions[0].fraction, None);
        assert_eq!(e.reference_amount, None);
        // a fixed share is somebody's 9,99 EUR of the bill however big the bill
        assert_eq!(e.transactions[1].amount, Some(999));
        assert!(e.validate().is_ok());
    }

    /// What the bank names beats everything the template implies, which is the whole reason a
    /// quarterly demand stops accruing over the quarter that follows it.
    #[test]
    fn a_period_on_the_statement_wins() {
        let account = Uuid::now_v7();
        let tpl = template(vec![transaction(account, Some(0))]);

        let d = ExpenseDelivery {
            date: at(2026, 8, 15),
            ..delivery(account, -5500, "RUNDFUNK ARD ZDF 07.2026 - 09.2026")
        };
        let e = impose(&tpl, &d, at(2026, 8, 15)).unwrap();

        assert_eq!(e.accrual_start, at(2026, 7, 1));
        assert_eq!(e.accrual_end, at(2026, 9, 30));
    }

    /// The template says a payment falls on the first day of the month it pays for, so a rent
    /// debited on the 1st is for that month...
    #[test]
    fn without_one_the_template_says_where_the_payment_sits() {
        let account = Uuid::now_v7();
        let mut tpl = template(vec![transaction(account, Some(0))]);
        tpl.accrual_start = at(2018, 10, 1);
        tpl.accrual_end = at(2018, 10, 31);
        tpl.transactions[0].date = at(2018, 10, 1);

        let d = ExpenseDelivery {
            date: at(2026, 11, 2),
            ..delivery(account, -95000, "DAUERAUFTRAG MIETE")
        };
        let e = impose(&tpl, &d, at(2026, 11, 2)).unwrap();

        assert_eq!(e.accrual_start, at(2026, 11, 1));
        assert_eq!(e.accrual_end, at(2026, 11, 30));
    }

    /// ...while a fee debited on the last day of the month it pays for keeps that month even
    /// when a bank holiday pushes the payment into the next one.
    #[test]
    fn a_payment_delayed_past_the_month_end_keeps_the_month_it_paid_for() {
        let account = Uuid::now_v7();
        let mut tpl = template(vec![transaction(account, Some(0))]);
        tpl.accrual_start = at(2024, 8, 1);
        tpl.accrual_end = at(2024, 8, 31);
        tpl.transactions[0].date = at(2024, 8, 31);

        let d = ExpenseDelivery {
            date: at(2026, 10, 2),
            ..delivery(account, -29000, "KITA BEITRAG")
        };
        let e = impose(&tpl, &d, at(2026, 10, 2)).unwrap();

        assert_eq!(e.accrual_start, at(2026, 9, 1));
        assert_eq!(e.accrual_end, at(2026, 9, 30));
    }

    /// The other transactions hang off the payment, not off the period: a template whose
    /// counter-booking follows its payment by a day keeps that day.
    #[test]
    fn the_counter_bookings_follow_the_payment() {
        let account = Uuid::now_v7();
        let other = Uuid::now_v7();
        let mut tpl = template(vec![transaction(account, Some(0)), {
            let mut t = transaction(other, Some(8000));
            t.date = at(2024, 8, 1);
            t
        }]);
        tpl.accrual_start = at(2024, 7, 1);
        tpl.accrual_end = at(2024, 7, 31);
        tpl.transactions[0].date = at(2024, 7, 31);

        let d = ExpenseDelivery {
            date: at(2026, 10, 30),
            ..delivery(account, -8000, "ABSCHLAG")
        };
        let e = impose(&tpl, &d, at(2026, 10, 30)).unwrap();

        assert_eq!(e.transactions[0].date, at(2026, 10, 30));
        assert_eq!(e.transactions[1].date, at(2026, 10, 31));
    }

    /// A one-day template is a purchase, and a purchase is for the day it happened wherever
    /// the delivery lands.
    #[test]
    fn a_single_day_template_still_accrues_on_the_day_it_was_paid() {
        let account = Uuid::now_v7();
        let tpl = template(vec![transaction(account, Some(-100))]);

        let e = impose(&tpl, &delivery(account, -4237, "REWE"), at(2025, 3, 5)).unwrap();

        assert_eq!(e.accrual_start, at(2025, 3, 4));
        assert_eq!(e.accrual_end, at(2025, 3, 4));
    }

    /// every generated expense goes through validate() before being stored, so imposing must
    /// not produce one that would be rejected
    #[test]
    fn imposed_expenses_are_valid() {
        let account = Uuid::now_v7();
        let tpl = template(vec![transaction(account, Some(-100))]);

        let e = impose(&tpl, &delivery(account, -4237, "x"), at(2025, 3, 5)).unwrap();

        assert!(e.validate().is_ok());
    }

    #[test]
    fn instantiated_expenses_are_valid() {
        let rule = periodic(PeriodicInterval::Monthly, at(2025, 3, 1));
        let tpl = template(vec![transaction(Uuid::now_v7(), Some(-500))]);

        let e = instantiate(&rule, &tpl, at(2025, 4, 15));

        assert!(e.validate().is_ok());
    }

    /// ties on priority must not depend on whatever order mongo hands the rules back in
    #[test]
    fn equal_priorities_resolve_deterministically() {
        let a = automation("aaa", 5);
        let b = automation("bbb", 5);
        let d = delivery(Uuid::now_v7(), -100, "x");

        let forwards = best_match(&[a.clone(), b.clone()], &d)
            .unwrap()
            .title
            .clone();
        let backwards = best_match(&[b, a], &d).unwrap().title.clone();

        assert_eq!(forwards, backwards);
    }

    /// all filters are ANDed: matching the statement is not enough if the amount disagrees
    #[test]
    fn filters_combine_conjunctively() {
        let account = Uuid::now_v7();
        let mut rule = automation("both", 1);
        rule.regex_statement = Some("REWE".into());
        rule.filter_amount = Some(-4237);

        assert!(matches(&rule, &delivery(account, -4237, "REWE SAGT DANKE")));
        assert!(!matches(&rule, &delivery(account, -100, "REWE SAGT DANKE")));
        assert!(!matches(&rule, &delivery(account, -4237, "ALDI")));
    }

    /// moneydb wrote its regexes against `dd.mm.yyyy HH:MM`; a different format would stop
    /// every imported regexTime rule from matching
    #[test]
    fn delivery_date_format_matches_moneydb() {
        let d = ExpenseDelivery {
            account_id: Uuid::now_v7(),
            date: Utc.with_ymd_and_hms(2025, 3, 4, 9, 5, 0).unwrap(),
            booking_date: None,
            amount: -100,
            statement: String::new(),
        };

        assert_eq!(d.formatted_date(), "04.03.2025 09:05");
    }

    #[test]
    fn imposing_a_delivery_overwrites_the_matching_transaction() {
        let account = Uuid::now_v7();
        let other = Uuid::now_v7();
        let tpl = template(vec![
            transaction(account, Some(-100)),
            transaction(other, Some(-100)),
        ]);

        let d = delivery(account, -4237, "LASTSCHRIFT REWE");
        let e = impose(&tpl, &d, at(2025, 3, 5)).unwrap();

        assert!(!e.is_template);
        assert!(e.is_unchecked);
        assert_eq!(e.accrual_start, d.date);

        let touched = e
            .transactions
            .iter()
            .find(|t| t.account_id == account)
            .unwrap();
        assert_eq!(touched.amount, Some(-4237));
        assert_eq!(touched.statement, "LASTSCHRIFT REWE");

        // the counter-booking is only shifted in time, its amount is the template's
        let untouched = e
            .transactions
            .iter()
            .find(|t| t.account_id == other)
            .unwrap();
        assert_eq!(untouched.amount, Some(-100));
    }

    #[test]
    fn imposing_a_delivery_sets_the_reference_amount_for_split_templates() {
        let account = Uuid::now_v7();
        let mut tpl = template(vec![
            transaction(account, None),
            transaction(Uuid::now_v7(), None),
        ]);
        tpl.transactions[0].fraction = Some(0.5);
        tpl.transactions[1].fraction = Some(0.5);
        tpl.reference_amount = Some(-100);

        let e = impose(&tpl, &delivery(account, -4237, "x"), at(2025, 3, 5)).unwrap();

        assert_eq!(e.reference_amount, Some(-4237));
    }
}
