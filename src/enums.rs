use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, DbEnum)]
#[DieselType = "Account_availability"]
pub enum AccountAvailability {
    Immediately,
    Days,
    Weeks,
    Months,
    Years,
    Decades,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, DbEnum)]
#[DieselType = "Account_risk"]
pub enum AccountRisk {
    None,
    Slight,
    Low,
    Medium,
    High,
    Huge,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, DbEnum)]
#[DieselType = "Account_kind"]
pub enum AccountKind {
    Cash,
    Debit,
    Credit,
    Debt,
    Stocks,
    Virtual,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, DbEnum)]
#[DieselType = "Expense_event_type"]
pub enum ExpenseEventType {
    Create,
    Modify,
    Delete,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, DbEnum)]
#[DieselType = "Expense_event_target"]
pub enum ExpenseEventTarget {
    Expense,
    Categories,
    Receipts,
    Transactions,
}
