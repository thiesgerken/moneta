//! Rules that create expenses from a template expense (one with `is_template` set).
//!
//! Two kinds, both inherited from moneydb: periodic rules fire on a schedule, automation
//! rules fire when a matching transaction is delivered from a bank statement.

use chrono::{DateTime, Datelike, TimeDelta, Timelike, Utc};
use diesel::prelude::*;
use diesel::{deserialize::FromSqlRow, expression::AsExpression};
use juniper::{GraphQLEnum, GraphQLInputObject, GraphQLObject};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::text_enum;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, GraphQLEnum, AsExpression, FromSqlRow,
)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub enum PeriodicInterval {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

text_enum!(PeriodicInterval {
    Daily => "daily",
    Weekly => "weekly",
    Monthly => "monthly",
    Yearly => "yearly",
});

impl PeriodicInterval {
    /// The `n`th occurrence counted from `anchor`.
    ///
    /// Counted from the anchor rather than stepped one occurrence at a time. Monthly and
    /// yearly clamp to the end of a short month, so a rule anchored on the 31st still fires in
    /// February -- but stepping would carry that clamp forward for good, and the rule would go
    /// on firing on the 28th once it had seen one February.
    ///
    /// `None` once the result leaves the range a timestamp can hold.
    pub fn advance(&self, anchor: DateTime<Utc>, n: u32) -> Option<DateTime<Utc>> {
        match self {
            Self::Daily => anchor.checked_add_signed(TimeDelta::days(n.into())),
            Self::Weekly => anchor.checked_add_signed(TimeDelta::weeks(n.into())),
            Self::Monthly => crate::accrual::add_months(anchor, n),
            Self::Yearly => crate::accrual::add_months(anchor, n.checked_mul(12)?),
        }
    }
}

#[derive(Debug, Clone, GraphQLObject, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::periodic_rules, check_for_backend(diesel::pg::Pg))]
pub struct PeriodicRule {
    pub id: Uuid,

    pub user_id: Uuid,

    pub title: String,

    /// the expense this rule copies, always flagged `is_template`. It carries the amount too:
    /// an occurrence has no delivery to take one from, so the template is the only place the
    /// amount can be written down.
    pub template_id: Uuid,

    pub period: PeriodicInterval,

    /// replaces the template's title. `%d`, `%m` and `%y` are substituted with the day,
    /// month and year of the occurrence being generated.
    pub title_replacement: Option<String>,

    /// occurrences at or before this instant have already been generated
    pub last_creation: DateTime<Utc>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLInputObject)]
#[serde(rename_all = "camelCase")]
pub struct PeriodicRuleInput {
    pub title: String,
    pub template_id: Uuid,
    pub period: PeriodicInterval,
    pub title_replacement: Option<String>,
    pub last_creation: DateTime<Utc>,
}

impl PeriodicRuleInput {
    pub fn upgrade(self, id: Uuid, user_id: Uuid, now: DateTime<Utc>) -> PeriodicRule {
        PeriodicRule {
            id,
            user_id,
            title: self.title,
            template_id: self.template_id,
            period: self.period,
            title_replacement: self.title_replacement,
            last_creation: self.last_creation,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, GraphQLObject, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::automation_rules, check_for_backend(diesel::pg::Pg))]
pub struct AutomationRule {
    pub id: Uuid,

    pub user_id: Uuid,

    pub title: String,

    /// the expense this rule copies, always flagged `is_template`. Its booking says *when* a
    /// payment falls inside the period; the delivery says on which account and how much.
    pub template_id: Uuid,

    /// higher wins when several rules match the same delivery
    pub priority: i32,

    /// matched against the delivery date formatted as `dd.mm.yyyy HH:MM` (UTC)
    pub regex_time: Option<String>,
    /// matched against the statement text of the delivered transaction
    pub regex_statement: Option<String>,
    /// exact match against the delivered amount, in cents
    pub filter_amount: Option<i32>,
    /// exact match against the account the transaction was delivered for
    pub filter_account_id: Option<Uuid>,

    /// when this rule last produced an expense
    pub last_delivery: Option<DateTime<Utc>>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLInputObject)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRuleInput {
    pub title: String,
    pub template_id: Uuid,
    pub priority: i32,
    pub regex_time: Option<String>,
    pub regex_statement: Option<String>,
    pub filter_amount: Option<i32>,
    pub filter_account_id: Option<Uuid>,
}

impl AutomationRuleInput {
    pub fn upgrade(
        self,
        id: Uuid,
        user_id: Uuid,
        last_delivery: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> AutomationRule {
        AutomationRule {
            id,
            user_id,
            title: self.title,
            template_id: self.template_id,
            priority: self.priority,
            regex_time: self.regex_time,
            regex_statement: self.regex_statement,
            filter_amount: self.filter_amount,
            filter_account_id: self.filter_account_id,
            last_delivery,
            created_at: now,
            updated_at: now,
        }
    }
}

/// A transaction handed to moneta from outside (a bank statement importer), before it is
/// known which expense — if any — it belongs to.
#[derive(Debug, Clone, Serialize, Deserialize, GraphQLInputObject)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseDelivery {
    pub account_id: Uuid,
    /// when the money moved (Wertstellung), which is the date everything is filed under
    pub date: DateTime<Utc>,
    /// when the bank posted the line, where the statement says so. Kept on the transaction it
    /// produces, and used for nothing else: two statements of the same payment can disagree
    /// about it, so it must not decide what a thing cost or when.
    pub booking_date: Option<DateTime<Utc>>,
    /// in cents, using the same sign convention as expense transactions
    pub amount: i32,
    pub statement: String,
}

/// A delivery that has already been processed.
///
/// Bank statement uploads overlap — the same lines get handed over again on the next run —
/// so deliveries are remembered to keep re-uploads from duplicating expenses.
#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::seen_deliveries, check_for_backend(diesel::pg::Pg))]
pub struct SeenDelivery {
    pub id: Uuid,

    pub user_id: Uuid,

    pub account_id: Uuid,
    pub date: DateTime<Utc>,
    pub amount: i32,
    pub statement: String,

    /// the expense this delivery produced, absent when no rule matched
    pub expense_id: Option<Uuid>,

    pub created_at: DateTime<Utc>,
}

impl ExpenseDelivery {
    /// The representation `regex_time` is matched against. Kept identical to moneydb's so
    /// imported rules keep matching what they used to.
    pub fn formatted_date(&self) -> String {
        format!(
            "{:02}.{:02}.{:04} {:02}:{:02}",
            self.date.day(),
            self.date.month(),
            self.date.year(),
            self.date.hour(),
            self.date.minute()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    const INTERVALS: [PeriodicInterval; 4] = [
        PeriodicInterval::Daily,
        PeriodicInterval::Weekly,
        PeriodicInterval::Monthly,
        PeriodicInterval::Yearly,
    ];

    fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 12, 0, 0).unwrap()
    }

    #[test]
    fn each_interval_advances_by_its_own_step() {
        assert_eq!(
            PeriodicInterval::Daily.advance(at(2026, 3, 1), 1).unwrap(),
            at(2026, 3, 2)
        );
        assert_eq!(
            PeriodicInterval::Weekly.advance(at(2026, 3, 1), 1).unwrap(),
            at(2026, 3, 8)
        );
        assert_eq!(
            PeriodicInterval::Monthly
                .advance(at(2026, 3, 1), 1)
                .unwrap(),
            at(2026, 4, 1)
        );
        assert_eq!(
            PeriodicInterval::Yearly.advance(at(2026, 3, 1), 1).unwrap(),
            at(2027, 3, 1)
        );
    }

    /// A rule anchored on the 31st has to keep firing through the months that have no 31st,
    /// and has to land on the last day of them rather than slipping into the next month.
    #[test]
    fn a_month_end_anchor_clamps_instead_of_overflowing() {
        assert_eq!(
            PeriodicInterval::Monthly
                .advance(at(2026, 1, 31), 1)
                .unwrap(),
            at(2026, 2, 28)
        );
        assert_eq!(
            PeriodicInterval::Monthly
                .advance(at(2026, 3, 31), 1)
                .unwrap(),
            at(2026, 4, 30)
        );
        // a leap year still has a 29th to clamp onto
        assert_eq!(
            PeriodicInterval::Monthly
                .advance(at(2028, 1, 31), 1)
                .unwrap(),
            at(2028, 2, 29)
        );
        // and the same anchor a year later, which is the one that only bites every four years
        assert_eq!(
            PeriodicInterval::Yearly
                .advance(at(2028, 2, 29), 1)
                .unwrap(),
            at(2029, 2, 28)
        );
    }

    /// moneydb's format, which every imported automation rule's `regex_time` was written
    /// against. Widening a field or dropping a leading zero silently stops those matching, and
    /// a rule that no longer fires looks like a bank that stopped sending the line.
    #[test]
    fn a_delivery_date_is_formatted_the_way_moneydb_wrote_it() {
        let delivery = ExpenseDelivery {
            account_id: Uuid::nil(),
            date: Utc.with_ymd_and_hms(2026, 8, 4, 9, 5, 0).unwrap(),
            booking_date: None,
            amount: -1234,
            statement: String::new(),
        };

        assert_eq!(delivery.formatted_date(), "04.08.2026 09:05");
    }

    /// The same agreement as the account enums: the literals are what the column holds, and
    /// the CHECK constraint in the migration is the other half of it.
    #[test]
    fn every_interval_is_one_the_check_constraint_allows() {
        let schema = include_str!("../../migrations/2026-08-05-000001_initial_schema/up.sql");
        let rest = schema
            .split_once("periodic_rules_period_known")
            .expect("the constraint is gone from the migration")
            .1;
        let allowed = &rest[..rest.find(')').expect("the CHECK list closes")];

        for interval in INTERVALS {
            assert!(
                allowed.contains(&format!("'{}'", interval.as_str())),
                "the constraint does not allow '{}'",
                interval.as_str()
            );
        }

        assert_eq!(allowed.matches('\'').count() / 2, INTERVALS.len());
    }
}
