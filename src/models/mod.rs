mod account;
mod expense;

pub use account::{Account, AccountSynchronization};
pub use expense::{
    Expense, ExpenseCategorization, ExpensePosition, ExpenseReceipt, ExpenseTransaction,
};

use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(rename = "_id")]
    pub id: ObjectId,

    pub name: String,
    pub full_name: String,
    pub password_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    #[serde(rename = "_id")]
    pub id: ObjectId,

    pub user_id: ObjectId,
    pub name: String,
    pub description: String,
    pub color: Option<String>,
    pub parent: Option<ObjectId>,
    pub replaces: Vec<ObjectId>, // list of category ids from other users

    // desired max spending in this category per month. In the future this could become a list of limits and timespans (to accomodate for varying spending limits over time without changing past data)
    pub monthly_limit: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Balance {
    #[serde(rename = "_id")]
    pub id: ObjectId,

    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub date: DateTime<Utc>, // includes expenses with date < this value (i.e. what happened the same instant is not included)

    pub account_id: ObjectId,
    pub amount: i64, // cents
    pub comments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangelogEntry<T> {
    pub user_id: Option<ObjectId>,

    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub date: DateTime<Utc>,

    // name of the program that was used to make the change (e.g. web app, CLI, ...)
    pub tool: String,

    // whether this action was performed by a human or an automatic system
    pub automatic: bool,
    // if this action was performed by an automated (AI) system, an indication (between 0 and 1) about how sure this system was.
    // value should be ignored if the action was performed by a human
    pub confidence: f32,

    // how the entry looked before and after the change.
    // either one of these (but not both
    // representing the creation or deletion of the entry.
    pub old_state: Option<T>,
    pub new_state: Option<T>,
}

pub type ExpenseChangelogEntry = ChangelogEntry<Expense>;
pub type BalanceChangelogEntry = ChangelogEntry<Balance>;
