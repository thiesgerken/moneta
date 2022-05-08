use chrono::{DateTime, Utc};
use rocket_okapi::JsonSchema;
use serde::{Deserialize, Serialize};

use super::ObjectIdWrapper;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum AccountAvailability {
    Immediately,
    Days,
    Weeks,
    Months,
    Years,
    Decades,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum AccountRisk {
    None,
    Slight,
    Low,
    Medium,
    High,
    Huge,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    #[serde(rename = "_id", default)]
    pub id: ObjectIdWrapper,

    #[serde(default)]
    pub user_id: ObjectIdWrapper,

    pub name: String,
    pub description: String,
    pub color: Option<String>,
    pub iban: Option<String>,
    pub kind: AccountKind,
    pub availability: AccountAvailability,
    pub risk: AccountRisk,

    // when the account was opened and possibly closed. Used to determine whether to hide the account or not.
    // at its opening, it is implicitly assumed that the account had a balance of zero.
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    #[schemars(with = "DateTime<Utc>")]
    pub opened: DateTime<Utc>,
    #[serde(with = "crate::queries::optional_chrono_datetime_as_bson_datetime")]
    #[schemars(with = "Option<DateTime<Utc>>")]
    pub closed: Option<DateTime<Utc>>,

    // how to add balances of this account to the total balance, must be >0 and <=1
    pub ownership_fraction: f32,

    // of course this needs to be mirrored in the other account
    pub synchronization: Option<AccountSynchronization>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccountSynchronization {
    pub account_id: ObjectIdWrapper, // id of the other account
    pub invert: bool, // whether to count income in the original account as an expense in the synchronized account (and vice versa) or not
}
