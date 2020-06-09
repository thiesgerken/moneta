use crate::models::*;

use serde::{Deserialize, Serialize};

// The contents of these "Rendered" entities may have been altered to be viewed by the user that requests them

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedAccount {
    pub info: Account,
    pub synchronization: Option<AccountSynchronization>,
}

pub struct RenderedExpense {
    pub info: Expense,
    pub events: Vec<ExpenseEvent>,
    pub receipts: Vec<ExpenseReceipt>,
    pub categories: Vec<ExpenseCategory>,
    pub transactions: Vec<ExpenseTransaction>,
}

pub struct RenderedCategory {
    pub info: Category,
    pub replaces: Vec<Category>,
}

pub type RenderedBalance = Balance;
