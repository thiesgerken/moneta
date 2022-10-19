use std::error::Error;

use chrono::{DateTime, Utc};
use juniper::{GraphQLEnum, GraphQLObject, Object, GraphQLInputObject};
use serde::{Deserialize, Serialize};

use super::ObjectIdWrapper;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, GraphQLEnum)]
pub enum AccountAvailability {
    Immediately,
    Days,
    Weeks,
    Months,
    Years,
    Decades,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, GraphQLEnum)]
pub enum AccountRisk {
    None,
    Slight,
    Low,
    Medium,
    High,
    Huge,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, GraphQLEnum)]
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

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
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
    pub opened: DateTime<Utc>,
    #[serde(with = "crate::queries::optional_chrono_datetime_as_bson_datetime")]
    pub closed: Option<DateTime<Utc>>,

    // how to add balances of this account to the total balance, must be >0 and <=1
    pub ownership_fraction: f64,

    // of course this needs to be mirrored in the other account
    pub synchronization: Option<AccountSynchronization>,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLInputObject)]
#[serde(rename_all = "camelCase")]
pub struct AccountInput {
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
    pub opened: DateTime<Utc>,
    #[serde(with = "crate::queries::optional_chrono_datetime_as_bson_datetime")]
    pub closed: Option<DateTime<Utc>>,

    // how to add balances of this account to the total balance, must be >0 and <=1
    pub ownership_fraction: f64,
}

pub type AccountUpdateInput = AccountInput;
pub type AccountCreateInput = AccountInput;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, GraphQLObject)]
#[serde(rename_all = "camelCase")]
pub struct AccountSynchronization {
    pub account_id: ObjectIdWrapper, // id of the other account
    pub invert: bool, // whether to count income in the original account as an expense in the synchronized account (and vice versa) or not
}

impl AccountInput {
    pub fn validate(&self) -> Result<(), Box<dyn Error>> {
        if let Some(closed) = self.closed {
            if closed <= self.opened {
                return Err("Account opening date must not be after account close date".into());
            }
        }

        if self.ownership_fraction <= 0.0 || self.ownership_fraction > 1.0 {
            return Err(
                "Account ownership fraction must be positive and less than (or equal to) 1.0"
                    .into(),
            );
        }

        Ok(())
    }

    pub fn upgrade(self, id: ObjectIdWrapper, user_id: ObjectIdWrapper) -> Account {
        Account {
            id,
            name: self.name,
            description: self.description,
            color: self.color,
            user_id: user_id,
            synchronization: None,
            iban: self.iban,
            kind: self.kind,
            availability: self.availability,
            risk: self.risk,
            opened: self.opened,
            closed: self.closed,
            ownership_fraction: self.ownership_fraction,
        }
    }
}
