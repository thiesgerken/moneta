use chrono::{DateTime, Utc};
use rocket_okapi::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{ExpenseCategorization, ExpensePosition, ExpenseReceipt, ObjectIdWrapper};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderedExpense {
    #[serde(rename = "_id", default)]
    pub id: ObjectIdWrapper,

    #[serde(default)]
    pub user_id: ObjectIdWrapper,

    pub title: String,
    pub store: String,
    pub description: String,
    pub comments: String,

    // #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub booking_start: DateTime<Utc>,
    // #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub booking_end: DateTime<Utc>,

    /// TODO: (not implemented yet)
    /// if this is part of something recurring, e.g. rent oder insurance payments,
    /// then this contains the id of the rule that was triggered / this expense belongs to.
    /// useful to check that these recurring expense have been paid yet in the current timespan and to spot mistakes (e.g. double payments)
    pub recurring_expense_id: Option<ObjectIdWrapper>,

    /// set to true if this expense is only a stanza for a recurring expense; this means it will not be counted in any analysis
    pub is_template: bool,
    /// to track expenses that should be looked at again when preparing tax forms
    pub is_tax_relevant: bool,
    /// needs to be looked at by a user first (missing info, low confidence, ...)
    pub is_unchecked: bool,
    /// if we anticipate more transactions to appear before this can be completed; e.g. payback when a order hasn't arrived
    pub is_incomplete: bool,

    pub receipts: Vec<ExpenseReceipt>,
    pub categories: Vec<ExpenseCategorization>,
    pub transactions: Vec<RenderedExpenseTransaction>,
    pub positions: Vec<ExpensePosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderedExpenseTransaction {
    pub account_id: ObjectIdWrapper,
    // #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub date: DateTime<Utc>,

    pub amount: Option<i64>,
    pub fraction: Option<f64>,

    pub comments: String,
    pub statement: String,

    pub calculated_amount: f64,
}
