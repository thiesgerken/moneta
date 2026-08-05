//! Turning rules into expenses.
//!
//! The interesting parts here are pure so they can be tested without a database: which
//! instants a periodic rule is due for, what the generated expense looks like, and which
//! automation rule a delivered transaction matches.

use std::error::Error;

use chrono::{DateTime, Datelike, TimeDelta, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use regex::Regex;
use uuid::Uuid;

use crate::models::{
    AutomationRule, Expense, ExpenseChangelogEntry, ExpenseDelivery, Json, PeriodicRule,
    SeenDelivery,
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
    e.id = Uuid::now_v7();
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
/// data onto the template.
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
    e.booking_start = delivery.date;
    e.booking_end = delivery.date + (template.booking_end - template.booking_start);
    e.created_at = now;
    e.updated_at = now;

    let shift = delivery.date - template.booking_start;
    for t in e.transactions.iter_mut() {
        t.date += shift;
    }

    // The delivered line replaces the template's own booking against that account. Without
    // such a booking there is nowhere to put the delivered amount, and carrying on would
    // produce an expense holding the template's amounts instead of the real ones — wrong
    // money, silently. That means the rule is pointed at the wrong template, so refuse.
    let Some(t) = e
        .transactions
        .iter_mut()
        .find(|t| t.account_id == delivery.account_id)
    else {
        return Err(format!(
            "template {:?} books nothing against account {:?}, so the delivered amount has nowhere to go",
            template.id, delivery.account_id
        )
        .into());
    };

    t.amount = Some(delivery.amount);
    t.fraction = None;
    t.date = delivery.date;
    t.statement = delivery.statement.clone();

    // the remaining fractional transactions are shares of the delivered amount
    if e.transactions.iter().any(|t| t.fraction.is_some()) {
        e.reference_amount = Some(delivery.amount);
    }

    Ok(e)
}

/// How far apart two otherwise identical bank lines have to be to count as separate
/// transactions. Statements carry no stable per-line id, so this is the only thing standing
/// between a re-upload and a duplicated expense.
///
/// Inherited from moneydb. It does mean a genuine second identical payment on the same day
/// is treated as a duplicate, which is the safer way round to be wrong.
pub const DELIVERY_DEDUP_WINDOW: TimeDelta = TimeDelta::hours(24);

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
        .filter(seen_deliveries::date.ge(now - TimeDelta::days(400)))
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

/// Runs every periodic rule that is due, writing the generated expenses and their changelog
/// entries, then advancing each rule's `last_creation`.
///
/// Idempotent: running it twice in a row generates nothing the second time.
pub async fn generate_periodic_expenses(
    conn: &mut AsyncPgConnection,
    now: DateTime<Utc>,
) -> Result<usize, Box<dyn Error + Send + Sync>> {
    let rules = periodic_rules::table
        .select(PeriodicRule::as_select())
        .load(conn)
        .await?;
    if rules.is_empty() {
        return Ok(0);
    }

    let until = now + PERIODIC_ADVANCE;
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

        for t in &times {
            let expense = instantiate(rule, &template, *t);
            if let Err(e) = expense.validate() {
                warn!(
                    "periodic rule {} ({}) would produce an invalid expense, skipping: {}",
                    rule.id, rule.title, e
                );
                continue;
            }
            generated.push((expense, rule.user_id));
        }

        advanced.push((rule.id, *times.last().unwrap()));
    }

    if generated.is_empty() {
        return Ok(0);
    }

    let count = generated.len();

    // One transaction for the whole run: an expense without its changelog entry, or a rule
    // advanced past expenses that were never written, both leave the books wrong in a way
    // nothing later notices.
    conn.transaction::<_, diesel::result::Error, _>(async |conn| {
        for (expense, user_id) in &generated {
            queries::insert_expense(conn, expense).await?;
            diesel::insert_into(expense_changes::table)
                .values(changelog_entry(expense, *user_id, "moneta-periodic"))
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

        Ok(())
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

    fn transaction(account_id: Uuid, amount: Option<i32>) -> ExpenseTransaction {
        ExpenseTransaction {
            account_id,
            date: at(2025, 1, 15),
            amount,
            fraction: None,
            comments: String::new(),
            statement: String::new(),
        }
    }

    fn periodic(period: PeriodicInterval, last_creation: DateTime<Utc>) -> PeriodicRule {
        PeriodicRule {
            id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            title: "Rundfunkgebühr".into(),
            template_id: Uuid::now_v7(),
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
        assert_eq!(e.booking_start, at(2025, 4, 15));
    }

    /// a template booked across several days keeps that span, and its transactions move with it
    #[test]
    fn instantiation_shifts_the_whole_booking_window() {
        let rule = periodic(PeriodicInterval::Monthly, at(2025, 3, 1));
        let mut tpl = template(vec![transaction(Uuid::now_v7(), Some(-500))]);
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

        let tpl = template(vec![transaction(Uuid::now_v7(), Some(-500))]);
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
            transaction(Uuid::now_v7(), Some(-1750)),
            transaction(Uuid::now_v7(), None),
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
            transaction(Uuid::now_v7(), None),
            transaction(Uuid::now_v7(), None),
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

    #[test]
    fn imposing_a_delivery_for_an_unbooked_account_is_refused() {
        let tpl = template(vec![transaction(Uuid::now_v7(), Some(-100))]);

        let err = impose(&tpl, &delivery(Uuid::now_v7(), -4237, "x"), at(2025, 3, 5));

        assert!(err.is_err());
    }

    /// the counter-bookings are shares of the delivered amount, so they must stay fractional
    /// and follow the new reference amount rather than keeping the template's numbers
    #[test]
    fn imposing_a_delivery_rescales_the_shares() {
        let account = Uuid::now_v7();
        let mut tpl = template(vec![
            transaction(account, Some(-1000)),
            transaction(Uuid::now_v7(), None),
        ]);
        tpl.transactions[1].fraction = Some(-0.5);
        tpl.reference_amount = Some(-1000);

        let e = impose(&tpl, &delivery(account, -4237, "REWE"), at(2025, 3, 5)).unwrap();

        assert_eq!(e.transactions[0].amount, Some(-4237));
        assert_eq!(e.reference_amount, Some(-4237));
        assert_eq!(e.transactions[1].fraction, Some(-0.5));
        assert_eq!(e.transactions[1].amount, None);
    }

    /// a template booking the delivered account fractionally becomes a fixed booking, and
    /// the result still has to satisfy the expense invariants
    #[test]
    fn imposing_a_delivery_onto_a_fractional_booking_stays_valid() {
        let account = Uuid::now_v7();
        let mut tpl = template(vec![
            transaction(account, None),
            transaction(Uuid::now_v7(), None),
        ]);
        tpl.transactions[0].fraction = Some(1.0);
        tpl.transactions[1].fraction = Some(-0.5);
        tpl.reference_amount = Some(-1000);

        let e = impose(&tpl, &delivery(account, -4237, "x"), at(2025, 3, 5)).unwrap();

        assert_eq!(e.transactions[0].amount, Some(-4237));
        assert_eq!(e.transactions[0].fraction, None);
        assert!(e.validate().is_ok());
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
        assert_eq!(e.booking_start, d.date);

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
