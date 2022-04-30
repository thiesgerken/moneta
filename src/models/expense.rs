use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{ObjectIdWrapper, Category, Account};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Expense {
    #[serde(rename = "_id")]
    pub id: ObjectIdWrapper,

    pub title: String,
    pub store: String,
    pub description: String,
    pub comments: String,

    pub booking_start: DateTime<Utc>,
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
    pub transactions: Vec<ExpenseTransaction>,
    pub positions: Vec<ExpensePosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedExpense {
    #[serde(rename = "_id")]
    pub id: ObjectIdWrapper,

    pub title: String,
    pub store: String,
    pub description: String,
    pub comments: String,

    pub booking_start: DateTime<Utc>,
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
    pub categories: Vec<RenderedExpenseCategorization>,
    pub transactions: Vec<RenderedExpenseTransaction>,
    pub positions: Vec<ExpensePosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseTransaction {
    pub account_id: ObjectIdWrapper,
    pub date: DateTime<Utc>,

    // constraint: amount is null OR fraction is null
    // constraint: fraction can only be used if the expense has exactly one transaction with an amount
    // convention: amounts are _added_ to account balances, i.e. a negative amount is a expense
    // convention: fractions are calculated relative to the sum of all transactions with a fixed amount
    pub amount: Option<i64>,
    pub fraction: Option<f64>,

    pub comments: String,
    pub statement: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedExpenseTransaction {
    pub account: Account,
    pub date: DateTime<Utc>,

    // constraint: amount is null OR fraction is null
    // constraint: fraction can only be used if the expense has exactly one transaction with an amount
    // convention: amounts are _added_ to account balances, i.e. a negative amount is a expense
    // convention: fractions are calculated relative to the sum of all transactions with a fixed amount
    pub amount: Option<i64>,
    pub fraction: Option<f64>,

    pub comments: String,
    pub statement: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseCategorization {
    pub category_id: ObjectIdWrapper,
    // constraint: weight > 0
    pub weight: f64,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpensePosition {
    pub amount: Option<i64>,
    pub units: i32,
    pub title: String,
    pub comments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseReceipt {
    pub file_name: String,
    pub ocr: Option<String>,
}
