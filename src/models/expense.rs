use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Expense {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub title: String,
    pub store: String,
    pub description: String,
    pub comments: String,
    pub booking_start: DateTime<Utc>,
    pub booking_end: DateTime<Utc>,
    pub group_id: Option<ObjectId>,
    pub is_template: bool,
    pub is_preliminary: bool,
    pub is_tax_relevant: bool,
    pub is_unchecked: bool,
    pub receipts: Vec<ExpenseReceipt>,
    pub categories: Vec<ExpenseCategorization>,
    pub transactions: Vec<ExpenseTransaction>,
    pub positions: Vec<ExpensePosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseTransaction {
    pub account_id: ObjectId,
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
    pub category_id: ObjectId,
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
