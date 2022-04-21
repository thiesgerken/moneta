mod account;
mod expense;

pub use account::{Account, AccountSynchronization};
pub use expense::{
    Expense, ExpenseCategorization, ExpensePosition, ExpenseReceipt, ExpenseTransaction,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub full_name: String,
    pub password_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub description: String,
    pub color: Option<String>,
    pub parent: Option<i32>,
    pub replaces: Vec<i32>, // list of category ids from other users
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Balance {
    pub id: i32,
    pub account_id: i32,
    pub date: DateTime<Utc>, // includes expenses with date < this value (i.e. what happened the same instant is not included)
    pub amount: i64,         // cents
    pub comments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangelogEntry<T> {
    pub user_id: Option<i32>,
    pub date: DateTime<Utc>,
    pub tool: String,
    pub automatic: bool,
    pub old_state: Option<T>,
    pub new_state: Option<T>,
}

pub type ExpenseChangelogEntry = ChangelogEntry<Expense>;
pub type BalanceChangelogEntry = ChangelogEntry<Balance>;
