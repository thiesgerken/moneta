use std::error::Error;

use chrono::{DateTime, Utc};
use juniper::{GraphQLInputObject, GraphQLObject};
use serde::{Deserialize, Serialize};

use super::ObjectIdWrapper;

impl Expense {
    pub fn validate(&self) -> Result<(), Box<dyn Error>> {
        if self.booking_start > self.booking_end {
            return Err("Expense booking start date must not be after booking end".into());
        }

        if self.categories.iter().any(|cw| cw.weight <= 0.0) {
            return Err("Expense categorizations must have positive weights".into());
        }

        if self.positions.iter().any(|p| p.units <= 0.0) {
            return Err("Expense positions must have positive units".into());
        }

        if self.transactions.is_empty() {
            return Err("Expense must have at least one transaction".into());
        }

        if self
            .transactions
            .iter()
            .any(|p| !(p.amount.is_some() ^ p.fraction.is_some()))
        {
            return Err("Expense transactions must have amount xor fraction set".into());
        }

        if self
            .transactions
            .iter()
            .any(|p| p.fraction.map(|x| x == 0.0).unwrap_or(false))
        {
            return Err("Expense transactions must not contain 0.0 as fraction".into());
        }

        if self.transactions.iter().any(|p| p.fraction.is_some()) && self.reference_amount.is_none()
        {
            return Err(
                "Expenses with at least one fractional transaction must contain a reference amount"
                    .into(),
            );
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[serde(rename_all = "camelCase")]
pub struct Expense {
    #[serde(rename = "_id", default)]
    pub id: ObjectIdWrapper,

    pub title: String,
    pub store: String,
    pub description: String,
    pub comments: String,

    /// reference for fractional transactions, can be None if only fixed transactions are used
    pub reference_amount: Option<i32>,

    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub booking_start: DateTime<Utc>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub booking_end: DateTime<Utc>,

    /// set to true if this expense is only a stanza for a recurring expense; this means it will not be counted in any analysis
    pub is_template: bool,
    /// to track expenses that should be looked at again when preparing tax forms
    pub is_tax_relevant: bool,
    /// needs to be looked at by a user first (missing info, low confidence, ...)
    pub is_unchecked: bool,
    /// if we anticipate more transactions to appear before this can be completed; e.g. payback when a order hasn't arrived
    pub is_incomplete: bool,
    /// if this expense should not be considered for monthly spending and so on
    pub is_extraordinary: bool,

    pub receipts: Vec<ExpenseReceipt>,
    pub categories: Vec<ExpenseCategorization>,
    pub transactions: Vec<ExpenseTransaction>, // must not be empty
    pub positions: Vec<ExpensePosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseTransaction {
    pub account_id: ObjectIdWrapper,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub date: DateTime<Utc>,

    // constraint: amount is null OR fraction is null
    // constraint: fraction can only be used if the expense has exactly one transaction with an amount
    // convention: amounts are _added_ to account balances, i.e. a negative amount is a expense
    // convention: fractions are calculated relative to the sum of all transactions with a fixed amount
    pub amount: Option<i32>,
    pub fraction: Option<f64>,

    pub comments: String,
    pub statement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseCategorization {
    pub category_id: ObjectIdWrapper,
    // constraint: weight > 0
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[serde(rename_all = "camelCase")]
pub struct ExpensePosition {
    pub amount: Option<i32>,
    pub units: f64,
    pub title: String,
    pub comments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseReceipt {
    pub uri: String, /// can be a local file or e.g. a reference to a document in a dms
    pub ocr: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLInputObject)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseTransactionInput {
    pub account_id: ObjectIdWrapper,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub date: DateTime<Utc>,

    // constraint: amount is null OR fraction is null
    // constraint: fraction can only be used if the expense has exactly one transaction with an amount
    // convention: amounts are _added_ to account balances, i.e. a negative amount is a expense
    // convention: fractions are calculated relative to the sum of all transactions with a fixed amount
    pub amount: Option<i32>,
    pub fraction: Option<f64>,

    pub comments: String,
    pub statement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLInputObject)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseCategorizationInput {
    pub category_id: ObjectIdWrapper,
    // constraint: weight > 0
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLInputObject)]
#[serde(rename_all = "camelCase")]
pub struct ExpensePositionInput {
    pub amount: Option<i32>,
    pub units: f64,
    pub title: String,
    pub comments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLInputObject)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseReceiptInput {
    pub uri: String,
    pub ocr: Option<String>,
}

impl ExpenseTransactionInput {
    pub fn upgrade(self) -> ExpenseTransaction {
        ExpenseTransaction {
            account_id: self.account_id,
            date: self.date,
            amount: self.amount,
            fraction: self.fraction,
            comments: self.comments,
            statement: self.statement,
        }
    }
}

impl ExpenseCategorizationInput {
    pub fn upgrade(self) -> ExpenseCategorization {
        ExpenseCategorization {
            category_id: self.category_id,
            weight: self.weight,
        }
    }
}
impl ExpensePositionInput {
    pub fn upgrade(self) -> ExpensePosition {
        ExpensePosition {
            amount: self.amount,
            units: self.units,
            title: self.title,
            comments: self.comments,
        }
    }
}

impl ExpenseReceiptInput {
    pub fn upgrade(self) -> ExpenseReceipt {
        ExpenseReceipt {
            uri: self.uri,
            ocr: self.ocr,
        }
    }
}
