//! Turning rules into expenses.
//!
//! The interesting parts here are pure so they can be tested without a database: which
//! instants a periodic rule is due for, what the generated expense looks like, and which
//! automation rule a delivered transaction matches.

use chrono::{DateTime, Datelike, TimeDelta, Utc};
use futures::TryStreamExt;
use mongodb::bson::{self, doc, oid::ObjectId};
use regex::Regex;
use std::error::Error;

use crate::models::{
    Account, AutomationRule, Expense, ExpenseChangelogEntry, ExpenseDelivery, PeriodicRule,
    rendering::RenderedExpense,
};

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
    let mut t = template.booking_start;

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
    let shift = at - template.booking_start;

    let mut e = template.clone();
    e.id = ObjectId::new().into();
    e.is_template = false;
    e.booking_start = at;
    e.booking_end = template.booking_end + shift;
    e.created_at = at;
    e.updated_at = at;

    for t in e.transactions.iter_mut() {
        t.date += shift;
    }

    if let Some(title) = &rule.title_replacement {
        e.title = substitute_date(title, at);
    }

    if let Some(amount) = rule.amount_replacement {
        e.reference_amount = Some(amount);

        // The reference amount only drives fractional transactions, so the fixed one — the
        // actual booking, usually paired with fractional counter-bookings for the shares —
        // has to be replaced too. With several fixed transactions there is no way to tell
        // how to divide the new amount, so those are left to the reference amount alone.
        let mut fixed = e.transactions.iter_mut().filter(|t| t.amount.is_some());
        if let (Some(t), None) = (fixed.next(), fixed.next()) {
            t.amount = Some(amount);
        }
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
pub fn changelog_entry(expense: &Expense, user_id: ObjectId, tool: &str) -> ExpenseChangelogEntry {
    ExpenseChangelogEntry {
        id: ObjectId::new().into(),
        target_id: expense.id,
        user_id: Some(user_id.into()),
        date: expense.created_at,
        tool: Some(tool.to_owned()),
        automatic: true,
        confidence: 1.0,
        old_state: None,
        new_state: Some(expense.clone()),
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
/// data onto the template.
pub fn impose(template: &Expense, delivery: &ExpenseDelivery, now: DateTime<Utc>) -> Expense {
    let mut e = template.clone();
    e.id = ObjectId::new().into();
    e.is_template = false;
    // the delivery is a bank statement line, so it is worth a human look before it counts
    e.is_unchecked = true;
    e.booking_start = delivery.date;
    e.booking_end = delivery.date + (template.booking_end - template.booking_start);
    e.created_at = now;
    e.updated_at = now;

    let shift = delivery.date - template.booking_start;
    for t in e.transactions.iter_mut() {
        t.date += shift;
    }

    // the delivered line replaces the template's own booking against that account
    if let Some(t) = e
        .transactions
        .iter_mut()
        .find(|t| t.account_id == delivery.account_id)
    {
        t.amount = Some(delivery.amount);
        t.fraction = None;
        t.date = delivery.date;
        t.statement = delivery.statement.clone();
    }

    if e.transactions.iter().any(|t| t.fraction.is_some()) {
        e.reference_amount = Some(delivery.amount);
    }

    e
}

/// How far ahead of `now` periodic expenses are generated, so a rule due later today is
/// already visible.
pub const PERIODIC_ADVANCE: TimeDelta = TimeDelta::hours(12);

/// Runs every periodic rule that is due, writing the generated expenses, their changelog
/// entries and their rendered documents, then advancing each rule's `last_creation`.
///
/// Idempotent: running it twice in a row generates nothing the second time.
pub async fn generate_periodic_expenses(
    connection: &mongodb::Database,
    now: DateTime<Utc>,
) -> Result<usize, Box<dyn Error>> {
    let rule_collection = connection.collection::<PeriodicRule>("periodic_rules");
    let expense_collection = connection.collection::<Expense>("expenses");

    let rules = rule_collection
        .find(doc!())
        .await?
        .try_collect::<Vec<_>>()
        .await?;
    if rules.is_empty() {
        return Ok(0);
    }

    let accounts = connection
        .collection::<Account>("accounts")
        .find(doc!())
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    let until = now + PERIODIC_ADVANCE;
    let mut generated = Vec::new();
    let mut advanced = Vec::new();

    for rule in &rules {
        let Some(template) = expense_collection
            .find_one(doc!("_id": *rule.template_id))
            .await?
        else {
            warn!(
                "periodic rule {:?} ({}) points at a template that no longer exists, skipping",
                rule.id, rule.title
            );
            continue;
        };

        let times = occurrences(rule, &template, until);
        if times.is_empty() {
            continue;
        }

        for t in &times {
            let expense = instantiate(rule, &template, *t);
            if let Err(e) = expense.validate() {
                warn!(
                    "periodic rule {:?} ({}) would produce an invalid expense, skipping: {}",
                    rule.id, rule.title, e
                );
                continue;
            }
            generated.push((expense, *rule.user_id));
        }

        advanced.push((rule.id, *times.last().unwrap()));
    }

    if generated.is_empty() {
        return Ok(0);
    }

    let expenses: Vec<_> = generated.iter().map(|(e, _)| e.clone()).collect();
    let changelog: Vec<_> = generated
        .iter()
        .map(|(e, uid)| changelog_entry(e, *uid, "moneta-periodic"))
        .collect();
    let rendered: Vec<_> = expenses
        .iter()
        .map(|e| e.render(&accounts))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();

    expense_collection.insert_many(&expenses).await?;
    connection
        .collection::<ExpenseChangelogEntry>("expense_changes")
        .insert_many(&changelog)
        .await?;
    if !rendered.is_empty() {
        connection
            .collection::<RenderedExpense>("expenses_rendered")
            .insert_many(&rendered)
            .await?;
    }

    // only advance once the expenses are safely stored, so a failure re-runs rather than
    // silently skipping a period
    for (id, last) in advanced {
        rule_collection
            .update_one(
                doc!("_id": *id),
                doc!("$set": {"lastCreation": bson::DateTime::from_chrono(last), "updatedAt": bson::DateTime::from_chrono(now)}),
            )
            .await?;
    }

    Ok(expenses.len())
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
            id: ObjectId::new().into(),
            title: "Rundfunkgebühr".into(),
            store: String::new(),
            description: String::new(),
            comments: String::new(),
            reference_amount: None,
            booking_start: at(2025, 1, 15),
            booking_end: at(2025, 1, 15),
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

    fn transaction(account_id: ObjectId, amount: Option<i32>) -> ExpenseTransaction {
        ExpenseTransaction {
            account_id: account_id.into(),
            date: at(2025, 1, 15),
            amount,
            fraction: None,
            comments: String::new(),
            statement: String::new(),
        }
    }

    fn periodic(period: PeriodicInterval, last_creation: DateTime<Utc>) -> PeriodicRule {
        PeriodicRule {
            id: ObjectId::new().into(),
            user_id: ObjectId::new().into(),
            title: "Rundfunkgebühr".into(),
            template_id: ObjectId::new().into(),
            period,
            title_replacement: None,
            amount_replacement: None,
            last_creation,
            created_at: at(2025, 1, 1),
            updated_at: at(2025, 1, 1),
        }
    }

    fn automation(title: &str, priority: i32) -> AutomationRule {
        AutomationRule {
            id: ObjectId::new().into(),
            user_id: ObjectId::new().into(),
            title: title.into(),
            template_id: ObjectId::new().into(),
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

    fn delivery(account_id: ObjectId, amount: i32, statement: &str) -> ExpenseDelivery {
        ExpenseDelivery {
            account_id: account_id.into(),
            date: at(2025, 3, 4),
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
        let tpl = template(vec![transaction(ObjectId::new(), Some(-1836))]);
        let e = instantiate(&rule, &tpl, at(2025, 4, 15));

        assert!(!e.is_template);
        assert_ne!(e.id, tpl.id);
        assert_eq!(e.booking_start, at(2025, 4, 15));
    }

    /// a template booked across several days keeps that span, and its transactions move with it
    #[test]
    fn instantiation_shifts_the_whole_booking_window() {
        let rule = periodic(PeriodicInterval::Monthly, at(2025, 3, 1));
        let mut tpl = template(vec![transaction(ObjectId::new(), Some(-500))]);
        tpl.booking_end = at(2025, 1, 18);

        let e = instantiate(&rule, &tpl, at(2025, 4, 15));

        assert_eq!(e.booking_start, at(2025, 4, 15));
        assert_eq!(e.booking_end, at(2025, 4, 18));
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
    fn amount_replacement_lands_on_a_single_fixed_transaction() {
        let mut rule = periodic(PeriodicInterval::Monthly, at(2025, 3, 1));
        rule.amount_replacement = Some(-1836);

        let tpl = template(vec![transaction(ObjectId::new(), Some(-500))]);
        let e = instantiate(&rule, &tpl, at(2025, 4, 15));

        assert_eq!(e.transactions[0].amount, Some(-1836));
        assert_eq!(e.reference_amount, Some(-1836));
    }

    /// the shape almost every shared expense imported from moneydb has: the real booking as
    /// a fixed amount, plus a fractional counter-booking for the other person's share
    #[test]
    fn amount_replacement_reaches_the_fixed_transaction_of_a_shared_expense() {
        let mut rule = periodic(PeriodicInterval::Monthly, at(2025, 3, 1));
        rule.amount_replacement = Some(-1836);

        let mut tpl = template(vec![
            transaction(ObjectId::new(), Some(-1750)),
            transaction(ObjectId::new(), None),
        ]);
        tpl.transactions[1].fraction = Some(-0.5);
        tpl.reference_amount = Some(-1750);

        let e = instantiate(&rule, &tpl, at(2025, 4, 15));

        assert_eq!(e.reference_amount, Some(-1836));
        assert_eq!(e.transactions[0].amount, Some(-1836));
        // the share follows from the reference amount, so it must stay fractional
        assert_eq!(e.transactions[1].amount, None);
        assert_eq!(e.transactions[1].fraction, Some(-0.5));
    }

    /// with several fixed transactions there is no way to split the new amount, so only the
    /// reference amount changes
    #[test]
    fn amount_replacement_leaves_split_transactions_alone() {
        let mut rule = periodic(PeriodicInterval::Monthly, at(2025, 3, 1));
        rule.amount_replacement = Some(-1000);

        let mut tpl = template(vec![
            transaction(ObjectId::new(), None),
            transaction(ObjectId::new(), None),
        ]);
        tpl.transactions[0].fraction = Some(0.5);
        tpl.transactions[1].fraction = Some(0.5);
        tpl.reference_amount = Some(-600);

        let e = instantiate(&rule, &tpl, at(2025, 4, 15));

        assert_eq!(e.reference_amount, Some(-1000));
        assert!(e.transactions.iter().all(|t| t.amount.is_none()));
        assert_eq!(e.transactions[0].fraction, Some(0.5));
    }

    #[test]
    fn a_rule_without_filters_matches_anything() {
        let rule = automation("catch all", 1);
        assert!(matches(&rule, &delivery(ObjectId::new(), -100, "whatever")));
    }

    #[test]
    fn statement_regex_must_match() {
        let mut rule = automation("Abfall", 1);
        rule.regex_statement = Some("Muster Stadtreinigung".into());

        assert!(matches(
            &rule,
            &delivery(
                ObjectId::new(),
                -100,
                "LASTSCHRIFT Muster Stadtreinigung AG"
            )
        ));
        assert!(!matches(
            &rule,
            &delivery(ObjectId::new(), -100, "REWE SAGT DANKE")
        ));
    }

    #[test]
    fn amount_and_account_filters_must_match_exactly() {
        let account = ObjectId::new();
        let mut rule = automation("exact", 1);
        rule.filter_amount = Some(-5001);
        rule.filter_account_id = Some(account.into());

        assert!(matches(&rule, &delivery(account, -5001, "x")));
        assert!(!matches(&rule, &delivery(account, -5000, "x")));
        assert!(!matches(&rule, &delivery(ObjectId::new(), -5001, "x")));
    }

    #[test]
    fn time_regex_matches_the_formatted_delivery_date() {
        let mut rule = automation("every fourth", 1);
        rule.regex_time = Some(r"^04\.03\.2025".into());

        assert!(matches(&rule, &delivery(ObjectId::new(), -100, "x")));
    }

    /// a rule nobody can parse must not silently become a catch-all
    #[test]
    fn an_invalid_regex_matches_nothing() {
        let mut rule = automation("broken", 1);
        rule.regex_statement = Some("([unclosed".into());

        assert!(!matches(
            &rule,
            &delivery(ObjectId::new(), -100, "anything")
        ));
    }

    #[test]
    fn the_highest_priority_matching_rule_wins() {
        let account = ObjectId::new();
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
                &delivery(ObjectId::new(), -100, "REWE")
            )
            .is_none()
        );
    }

    #[test]
    fn imposing_a_delivery_overwrites_the_matching_transaction() {
        let account = ObjectId::new();
        let other = ObjectId::new();
        let tpl = template(vec![
            transaction(account, Some(-100)),
            transaction(other, Some(-100)),
        ]);

        let d = delivery(account, -4237, "LASTSCHRIFT REWE");
        let e = impose(&tpl, &d, at(2025, 3, 5));

        assert!(!e.is_template);
        assert!(e.is_unchecked);
        assert_eq!(e.booking_start, d.date);

        let touched = e
            .transactions
            .iter()
            .find(|t| *t.account_id == account)
            .unwrap();
        assert_eq!(touched.amount, Some(-4237));
        assert_eq!(touched.statement, "LASTSCHRIFT REWE");

        // the counter-booking is only shifted in time, its amount is the template's
        let untouched = e
            .transactions
            .iter()
            .find(|t| *t.account_id == other)
            .unwrap();
        assert_eq!(untouched.amount, Some(-100));
    }

    #[test]
    fn imposing_a_delivery_sets_the_reference_amount_for_split_templates() {
        let account = ObjectId::new();
        let mut tpl = template(vec![
            transaction(account, None),
            transaction(ObjectId::new(), None),
        ]);
        tpl.transactions[0].fraction = Some(0.5);
        tpl.transactions[1].fraction = Some(0.5);
        tpl.reference_amount = Some(-100);

        let e = impose(&tpl, &delivery(account, -4237, "x"), at(2025, 3, 5));

        assert_eq!(e.reference_amount, Some(-4237));
    }
}
