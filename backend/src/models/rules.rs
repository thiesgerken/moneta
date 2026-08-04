//! Rules that create expenses from a template expense (one with `is_template` set).
//!
//! Two kinds, both inherited from moneydb: periodic rules fire on a schedule, automation
//! rules fire when a matching transaction is delivered from a bank statement.

use chrono::{DateTime, Datelike, Months, TimeDelta, Timelike, Utc};
use juniper::{GraphQLEnum, GraphQLInputObject, GraphQLObject};
use serde::{Deserialize, Serialize};

use super::ObjectIdWrapper;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, GraphQLEnum)]
pub enum PeriodicInterval {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl PeriodicInterval {
    /// The next occurrence after `t`. Monthly and yearly clamp to the end of the month, so a
    /// rule anchored on the 31st still fires in February.
    pub fn next(&self, t: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            Self::Daily => t + TimeDelta::days(1),
            Self::Weekly => t + TimeDelta::weeks(1),
            Self::Monthly => t + Months::new(1),
            Self::Yearly => t + Months::new(12),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[serde(rename_all = "camelCase")]
pub struct PeriodicRule {
    #[serde(rename = "_id", default)]
    pub id: ObjectIdWrapper,

    #[serde(default)]
    pub user_id: ObjectIdWrapper,

    pub title: String,

    /// the expense this rule copies. Often but not always flagged `is_template` — moneydb
    /// let ordinary past expenses double as prototypes, and the import preserves that.
    pub template_id: ObjectIdWrapper,

    pub period: PeriodicInterval,

    /// replaces the template's title. `%d`, `%m` and `%y` are substituted with the day,
    /// month and year of the occurrence being generated.
    pub title_replacement: Option<String>,

    /// replaces the template's reference amount, in cents
    pub amount_replacement: Option<i32>,

    /// occurrences at or before this instant have already been generated
    #[serde(with = "crate::queries::chrono_datetime_as_bson_datetime")]
    pub last_creation: DateTime<Utc>,

    #[serde(with = "crate::queries::chrono_datetime_as_bson_datetime")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "crate::queries::chrono_datetime_as_bson_datetime")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLInputObject)]
#[serde(rename_all = "camelCase")]
pub struct PeriodicRuleInput {
    pub title: String,
    pub template_id: ObjectIdWrapper,
    pub period: PeriodicInterval,
    pub title_replacement: Option<String>,
    pub amount_replacement: Option<i32>,
    pub last_creation: DateTime<Utc>,
}

impl PeriodicRuleInput {
    pub fn upgrade(
        self,
        id: ObjectIdWrapper,
        user_id: ObjectIdWrapper,
        now: DateTime<Utc>,
    ) -> PeriodicRule {
        PeriodicRule {
            id,
            user_id,
            title: self.title,
            template_id: self.template_id,
            period: self.period,
            title_replacement: self.title_replacement,
            amount_replacement: self.amount_replacement,
            last_creation: self.last_creation,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRule {
    #[serde(rename = "_id", default)]
    pub id: ObjectIdWrapper,

    #[serde(default)]
    pub user_id: ObjectIdWrapper,

    pub title: String,

    /// the expense this rule copies; see `PeriodicRule::template_id` on the `is_template` flag
    pub template_id: ObjectIdWrapper,

    /// higher wins when several rules match the same delivery
    pub priority: i32,

    /// matched against the delivery date formatted as `dd.mm.yyyy HH:MM` (UTC)
    pub regex_time: Option<String>,
    /// matched against the statement text of the delivered transaction
    pub regex_statement: Option<String>,
    /// exact match against the delivered amount, in cents
    pub filter_amount: Option<i32>,
    /// exact match against the account the transaction was delivered for
    pub filter_account_id: Option<ObjectIdWrapper>,

    /// when this rule last produced an expense
    #[serde(with = "crate::queries::optional_chrono_datetime_as_bson_datetime")]
    pub last_delivery: Option<DateTime<Utc>>,

    #[serde(with = "crate::queries::chrono_datetime_as_bson_datetime")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "crate::queries::chrono_datetime_as_bson_datetime")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLInputObject)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRuleInput {
    pub title: String,
    pub template_id: ObjectIdWrapper,
    pub priority: i32,
    pub regex_time: Option<String>,
    pub regex_statement: Option<String>,
    pub filter_amount: Option<i32>,
    pub filter_account_id: Option<ObjectIdWrapper>,
}

impl AutomationRuleInput {
    pub fn upgrade(
        self,
        id: ObjectIdWrapper,
        user_id: ObjectIdWrapper,
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
    pub account_id: ObjectIdWrapper,
    pub date: DateTime<Utc>,
    /// in cents, using the same sign convention as expense transactions
    pub amount: i32,
    pub statement: String,
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
