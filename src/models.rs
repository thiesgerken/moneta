use crate::enums::*;
use crate::schema::*;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Queryable, Insertable, Identifiable, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub full_name: String,
    pub hash: String,
}

#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[table_name = "users"]
pub struct NewUser {
    pub name: String,
    pub full_name: String,
    pub hash: String,
}

#[derive(
    Debug,
    Clone,
    Queryable,
    QueryableByName,
    Insertable,
    Associations,
    Identifiable,
    Serialize,
    Deserialize,
    AsChangeset,
)]
#[belongs_to(User, foreign_key = "user_id")]
#[serde(rename_all = "camelCase")]
#[table_name = "accounts"]
pub struct Account {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub description: String,
    pub color: Option<String>,
    pub iban: Option<String>,
    pub kind: AccountKind,
    pub availability: AccountAvailability,
    pub risk: AccountRisk,
    pub hidden: bool,
}

#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[table_name = "accounts"]
#[serde(rename_all = "camelCase")]
pub struct NewAccount {
    pub user_id: i32,
    pub name: String,
    pub description: String,
    pub color: Option<String>,
    pub iban: Option<String>,
    pub kind: AccountKind,
    pub availability: AccountAvailability,
    pub risk: AccountRisk,
    pub hidden: bool,
}

#[derive(
    Debug,
    Clone,
    Queryable,
    QueryableByName,
    Insertable,
    Associations,
    Identifiable,
    Serialize,
    Deserialize,
    AsChangeset,
)]
#[belongs_to(User, foreign_key = "user_id")]
#[table_name = "categories"]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub description: String,
    pub color: Option<String>,
    pub parent: Option<i32>,
}

#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[table_name = "categories"]
#[serde(rename_all = "camelCase")]
pub struct NewCategory {
    pub user_id: i32,
    pub name: String,
    pub description: String,
    pub color: Option<String>,
    pub parent: Option<i32>,
}

#[derive(
    Debug,
    Clone,
    Queryable,
    QueryableByName,
    Insertable,
    Associations,
    Identifiable,
    Serialize,
    Deserialize,
    AsChangeset,
)]
#[serde(rename_all = "camelCase")]
#[primary_key("account1")]
#[table_name = "account_synchronizations"]
pub struct AccountSynchronization {
    // constraint: account1 < account2, account1 and account2 are unique
    // also (not checked by db: each account can only be synced to at most one other account)
    pub account1: i32,
    pub account2: i32,
    pub user1: i32, // owner of account1. Workaround because otherwise a lot of queries would not be representable using diesel
    pub user2: i32, // owner of account2
    pub invert: bool,
}
#[derive(
    Debug,
    Clone,
    Queryable,
    QueryableByName,
    Insertable,
    Associations,
    Identifiable,
    Serialize,
    Deserialize,
    AsChangeset,
)]
#[serde(rename_all = "camelCase")]
#[primary_key("user_id", "original")]
#[table_name = "category_replacements"]
#[belongs_to(User, foreign_key = "user_id")]
pub struct CategoryReplacement {
    pub user_id: i32,
    pub original: i32,
    pub replacement: i32,
}

#[derive(
    Debug,
    Clone,
    Queryable,
    QueryableByName,
    Insertable,
    Associations,
    Identifiable,
    Serialize,
    Deserialize,
    AsChangeset,
)]
#[belongs_to(Account, foreign_key = "account_id")]
#[serde(rename_all = "camelCase")]
#[primary_key("id")]
#[table_name = "balances"]
pub struct Balance {
    pub id: i32,
    pub account_id: i32,
    pub date: DateTime<Utc>, // includes expenses with date < this value (i.e. what happened the same instant is not included)
    pub amount: i64,         // cents
    pub comment: String,
}

#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[table_name = "balances"]
#[serde(rename_all = "camelCase")]
pub struct NewBalance {
    pub account_id: i32,
    pub date: DateTime<Utc>, // includes expenses with date < this value (i.e. what happened the same instant is not included)
    pub amount: i64,         // cents
    pub comment: String,
}

#[derive(
    Debug,
    Clone,
    Queryable,
    QueryableByName,
    Insertable,
    Associations,
    Identifiable,
    Serialize,
    Deserialize,
    AsChangeset,
)]
#[serde(rename_all = "camelCase")]
#[table_name = "expenses"]
pub struct Expense {
    pub id: i32,
    pub title: String,
    pub description: String,
    pub store: String,
    pub comments: String,
    pub booking_start: DateTime<Utc>,
    pub booking_end: DateTime<Utc>,
    pub is_deleted: bool,
    pub is_template: bool,
    pub is_preliminary: bool,
    pub is_tax_relevant: bool,
    pub is_unchecked: bool,
}

#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[table_name = "expenses"]
#[serde(rename_all = "camelCase")]
pub struct NewExpense {
    pub title: String,
    pub description: String,
    pub store: String,
    pub comments: String,
    pub booking_start: DateTime<Utc>,
    pub booking_end: DateTime<Utc>,
    pub is_deleted: bool,
    pub is_template: bool,
    pub is_preliminary: bool,
    pub is_tax_relevant: bool,
    pub is_unchecked: bool,
}

#[derive(
    Debug,
    Clone,
    Queryable,
    QueryableByName,
    Insertable,
    Associations,
    Identifiable,
    Serialize,
    Deserialize,
    AsChangeset,
)]
#[belongs_to(Expense, foreign_key = "expense_id")]
#[table_name = "expense_transactions"]
#[serde(rename_all = "camelCase")]
pub struct ExpenseTransaction {
    pub id: i32,
    pub expense_id: i32,
    pub account_id: i32,
    pub date: DateTime<Utc>,
    // constraint: amount is null OR fraction is null
    // constraint: fraction can only be used if the expense has exactly one transaction with an amount
    pub amount: Option<i64>,
    pub fraction: Option<f64>,
    pub comments: String,
    pub statement: String,
}

#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[table_name = "expense_transactions"]
#[serde(rename_all = "camelCase")]
pub struct NewExpenseTransaction {
    pub expense_id: i32,
    pub account_id: i32,
    pub date: DateTime<Utc>,
    pub amount: Option<i64>,
    pub fraction: Option<f64>,
    pub comments: String,
    pub statement: String,
}

#[derive(
    Debug,
    Clone,
    Queryable,
    QueryableByName,
    Insertable,
    Associations,
    Identifiable,
    Serialize,
    Deserialize,
    AsChangeset,
)]
#[belongs_to(Expense, foreign_key = "expense_id")]
#[table_name = "expense_events"]
#[serde(rename_all = "camelCase")]
pub struct ExpenseEvent {
    pub id: i32,
    pub expense_id: i32,
    pub user_id: Option<i32>,
    pub date: DateTime<Utc>,
    pub tool: String,
    pub automatic: bool,
    pub event_type: ExpenseEventType,
    pub event_target: ExpenseEventTarget,
    pub payload: Option<String>,
}

#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[table_name = "expense_events"]
#[serde(rename_all = "camelCase")]
pub struct NewExpenseEvent {
    pub expense_id: i32,
    pub user_id: i32,
    pub date: DateTime<Utc>,
    pub tool: String,
    pub automatic: bool,
    pub event_type: ExpenseEventType,
    pub event_target: ExpenseEventTarget,
    pub payload: Option<String>,
}

#[derive(
    Debug,
    Clone,
    Queryable,
    QueryableByName,
    Insertable,
    Associations,
    Identifiable,
    Serialize,
    Deserialize,
    AsChangeset,
)]
#[belongs_to(Expense, foreign_key = "expense_id")]
#[serde(rename_all = "camelCase")]
#[primary_key("expense_id", "category_id")]
#[table_name = "expense_categories"]
pub struct ExpenseCategory {
    pub expense_id: i32,
    pub category_id: i32,
    // constraint: weight > 0
    pub weight: f64,
}

#[derive(
    Debug,
    Clone,
    Queryable,
    QueryableByName,
    Insertable,
    Associations,
    Identifiable,
    Serialize,
    Deserialize,
    AsChangeset,
)]
#[belongs_to(Expense, foreign_key = "expense_id")]
#[table_name = "expense_receipts"]
#[serde(rename_all = "camelCase")]
pub struct ExpenseReceipt {
    pub id: i32,
    pub expense_id: i32,
    pub file_name: String,
}

#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[table_name = "expense_receipts"]
#[serde(rename_all = "camelCase")]
pub struct NewExpenseReceipt {
    pub expense_id: i32,
    pub file_name: String,
}

#[derive(
    Debug,
    Clone,
    Queryable,
    Insertable,
    Associations,
    Identifiable,
    Serialize,
    Deserialize,
    AsChangeset,
)]
#[belongs_to(Expense, foreign_key = "template_id")]
#[belongs_to(User, foreign_key = "user_id")]
#[serde(rename_all = "camelCase")]
pub struct DeliveryRule {
    pub id: i32,
    pub user_id: i32,
    pub priority: i32,
    pub template_id: i32,
    pub account_id: Option<i32>,
    pub amount: Option<i64>,
    pub statement_regex: String,
    pub last_match: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[table_name = "delivery_rules"]
#[serde(rename_all = "camelCase")]
pub struct NewDeliveryRule {
    pub user_id: i32,
    pub priority: i32,
    pub template_id: i32,
    pub account_id: Option<i32>,
    pub amount: Option<i64>,
    pub statement_regex: String,
    pub last_match: Option<DateTime<Utc>>,
}

// TODO: Expense Delivery by API
// SeenExpense json sql=seen_expenses
//     valueDate     UTCTime
//     bookingDate   UTCTime Maybe
//     amount        Decimal
//     accountId     AccountId
//     preliminary   Bool
//     transaction   String
//     deriving Show Generic
// |]
