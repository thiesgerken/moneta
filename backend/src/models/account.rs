use std::error::Error;

use chrono::{DateTime, Utc};
use diesel::{deserialize::FromSqlRow, expression::AsExpression};
use juniper::{GraphQLEnum, GraphQLInputObject, GraphQLObject};
use serde::{Deserialize, Serialize};

use super::{ObjectIdWrapper, text_enum};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, GraphQLEnum, AsExpression, FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub enum AccountAvailability {
    Immediately,
    Days,
    Weeks,
    Months,
    Years,
    Decades,
}

text_enum!(AccountAvailability {
    Immediately => "immediately",
    Days => "days",
    Weeks => "weeks",
    Months => "months",
    Years => "years",
    Decades => "decades",
});

#[derive(Debug, Clone, Copy, Serialize, Deserialize, GraphQLEnum, AsExpression, FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub enum AccountRisk {
    None,
    Slight,
    Low,
    Medium,
    High,
    Huge,
}

text_enum!(AccountRisk {
    None => "none",
    Slight => "slight",
    Low => "low",
    Medium => "medium",
    High => "high",
    Huge => "huge",
});

#[derive(Debug, Clone, Copy, Serialize, Deserialize, GraphQLEnum, AsExpression, FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Text)]
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

text_enum!(AccountKind {
    Cash => "cash",
    Debit => "debit",
    Credit => "credit",
    Debt => "debt",
    Stocks => "stocks",
    RealEstate => "real_estate",
    Virtual => "virtual",
    Other => "other",
});

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
    #[serde(with = "crate::queries::chrono_datetime_as_bson_datetime")]
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
    pub name: String,
    pub description: String,
    pub color: Option<String>,
    pub iban: Option<String>,
    pub kind: AccountKind,
    pub availability: AccountAvailability,
    pub risk: AccountRisk,

    // when the account was opened and possibly closed. Used to determine whether to hide the account or not.
    // at its opening, it is implicitly assumed that the account had a balance of zero.
    #[serde(with = "crate::queries::chrono_datetime_as_bson_datetime")]
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
        if let Some(closed) = self.closed
            && closed <= self.opened
        {
            return Err("Account opening date must not be after account close date".into());
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
            user_id,
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
