use std::error::Error;
use uuid::Uuid;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{deserialize::FromSqlRow, expression::AsExpression};
use juniper::{GraphQLEnum, GraphQLInputObject, GraphQLObject};
use serde::{Deserialize, Serialize};

use super::text_enum;

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

#[derive(Debug, Clone, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::accounts, check_for_backend(diesel::pg::Pg))]
pub struct Account {
    pub id: Uuid,
    pub user_id: Uuid,

    pub name: String,
    pub description: String,
    pub color: Option<String>,
    pub iban: Option<String>,
    pub kind: AccountKind,
    pub availability: AccountAvailability,
    pub risk: AccountRisk,

    /// when the account was opened and possibly closed. Used to determine whether to hide the
    /// account or not. At its opening it is implicitly assumed to have had a balance of zero.
    pub opened: DateTime<Utc>,
    pub closed: Option<DateTime<Utc>>,

    /// how to add balances of this account to the total balance, must be >0 and <=1
    pub ownership_fraction: f64,

    /// The mirror of this account in another user's books, stored flat because that is what a
    /// row is. `synchronization()` puts the pair back together; the reverse link on the other
    /// account has to be kept in step by hand, since no constraint can see across rows.
    pub sync_account_id: Option<Uuid>,
    pub sync_invert: Option<bool>,
}

impl Account {
    pub fn synchronization(&self) -> Option<AccountSynchronization> {
        Some(AccountSynchronization {
            account_id: self.sync_account_id?,
            invert: self.sync_invert?,
        })
    }
}

#[derive(Debug, Clone, GraphQLInputObject)]
pub struct AccountInput {
    pub name: String,
    pub description: String,
    pub color: Option<String>,
    pub iban: Option<String>,
    pub kind: AccountKind,
    pub availability: AccountAvailability,
    pub risk: AccountRisk,

    pub opened: DateTime<Utc>,
    pub closed: Option<DateTime<Utc>>,

    pub ownership_fraction: f64,
}

pub type AccountUpdateInput = AccountInput;
pub type AccountCreateInput = AccountInput;

#[derive(Debug, Clone, PartialEq, Eq, GraphQLObject)]
pub struct AccountSynchronization {
    pub account_id: Uuid, // id of the other account
    /// whether to count income in the original account as an expense in the synchronized
    /// account (and vice versa) or not
    pub invert: bool,
}

impl AccountInput {
    pub fn validate(&self) -> Result<(), Box<dyn Error>> {
        // closed > opened and the ownership fraction range are CHECK constraints now; this
        // stays so the API reports them before the database does
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

    pub fn upgrade(self, id: Uuid, user_id: Uuid) -> Account {
        Account {
            id,
            user_id,
            name: self.name,
            description: self.description,
            color: self.color,
            iban: self.iban,
            kind: self.kind,
            availability: self.availability,
            risk: self.risk,
            opened: self.opened,
            closed: self.closed,
            ownership_fraction: self.ownership_fraction,
            sync_account_id: None,
            sync_invert: None,
        }
    }
}
