use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AccountAvailability {
    Immediately,
    Days,
    Weeks,
    Months,
    Years,
    Decades,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AccountRisk {
    None,
    Slight,
    Low,
    Medium,
    High,
    Huge,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AccountKind {
    Cash,
    Debit,
    Credit,
    Debt,
    Stocks,
    RealEstate,
    Virtual,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    pub opened: DateTime<Utc>,
    pub closed: Option<DateTime<Utc>>, // used to determine whether to hide the account or not
    pub ownership_fraction: f32, // how to add balances of this account to the total balance, must be >0 and <=1
    pub synchronization: Option<AccountSynchronization>, // of course this needs to be mirrored in the other account
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSynchronization {
    pub account_id: i32, // id of the other account
    pub invert: bool, // whether to count income in the original account as an expense in the synchronized account (and vice versa) or not
}
