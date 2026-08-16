use std::error::Error;
use uuid::Uuid;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{deserialize::FromSqlRow, expression::AsExpression};
use juniper::{GraphQLEnum, GraphQLInputObject, GraphQLObject};
use serde::{Deserialize, Serialize};

use super::text_enum;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    GraphQLEnum,
    AsExpression,
    FromSqlRow,
)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub enum AccountAvailability {
    Immediately,
    Days,
    Weeks,
    Months,
    Years,
    Decades,
}

impl AccountAvailability {
    /// Most liquid first. The variant order carries the meaning -- "available within days"
    /// includes anything available sooner -- and a caller grouping by it needs that order
    /// without depending on `HashMap` iteration.
    pub const ORDERED: [Self; 6] = [
        Self::Immediately,
        Self::Days,
        Self::Weeks,
        Self::Months,
        Self::Years,
        Self::Decades,
    ];
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

    /// The same depot over in stockdb, which knows what it is worth at today's prices. What
    /// that link is good for is `graphql::investments`, and it is read-only there: nothing on
    /// this side is ever written from it.
    pub stockdb_account_id: Option<i32>,
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

    pub stockdb_account_id: Option<i32>,
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
            stockdb_account_id: self.stockdb_account_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    /// The variants, spelled out, because nothing here can enumerate them for us. A variant
    /// added to one of these enums has to be added here too — the count check below is what
    /// says so, by disagreeing with the CHECK constraint the moment the two lists differ.
    const KINDS: [AccountKind; 8] = [
        AccountKind::Cash,
        AccountKind::Debit,
        AccountKind::Credit,
        AccountKind::Debt,
        AccountKind::Stocks,
        AccountKind::RealEstate,
        AccountKind::Virtual,
        AccountKind::Other,
    ];
    const RISKS: [AccountRisk; 6] = [
        AccountRisk::None,
        AccountRisk::Slight,
        AccountRisk::Low,
        AccountRisk::Medium,
        AccountRisk::High,
        AccountRisk::Huge,
    ];

    fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 12, 0, 0).unwrap()
    }

    fn input() -> AccountInput {
        AccountInput {
            name: "Girokonto".into(),
            description: String::new(),
            color: None,
            iban: None,
            kind: AccountKind::Debit,
            availability: AccountAvailability::Immediately,
            risk: AccountRisk::None,
            opened: at(2025, 1, 1),
            closed: None,
            ownership_fraction: 1.0,
            stockdb_account_id: None,
        }
    }

    #[test]
    fn accepts_an_open_account_owned_outright() {
        assert!(input().validate().is_ok());
    }

    #[test]
    fn accepts_a_closing_date_after_the_opening_one() {
        let account = AccountInput {
            closed: Some(at(2025, 6, 1)),
            ..input()
        };
        assert!(account.validate().is_ok());
    }

    #[test]
    fn rejects_a_closing_date_before_the_opening_one() {
        let account = AccountInput {
            closed: Some(at(2024, 6, 1)),
            ..input()
        };
        assert!(account.validate().is_err());
    }

    /// The constraint is `closed > opened`, so the two falling on the same instant is not a
    /// zero-length account but a contradiction — and the API has to say so in the same place
    /// the database would.
    #[test]
    fn rejects_a_closing_date_equal_to_the_opening_one() {
        let account = AccountInput {
            closed: Some(at(2025, 1, 1)),
            ..input()
        };
        assert!(account.validate().is_err());
    }

    #[test]
    fn accepts_a_half_owned_account() {
        let account = AccountInput {
            ownership_fraction: 0.5,
            ..input()
        };
        assert!(account.validate().is_ok());
    }

    /// Zero is the end worth pinning: an account nobody owns any of contributes nothing to
    /// anybody's wealth, so it renders as a column of zeroes rather than as an error.
    #[test]
    fn rejects_an_ownership_fraction_outside_zero_to_one() {
        for fraction in [0.0, -0.5, 1.5] {
            let account = AccountInput {
                ownership_fraction: fraction,
                ..input()
            };
            assert!(
                account.validate().is_err(),
                "ownership fraction {fraction} should have been rejected"
            );
        }
    }

    /// The literals are the on-disk representation and the CHECK constraint is the other half
    /// of that agreement. A mismatch surfaces the first time a row is written, which is a long
    /// way from where the mistake was made — so the migration is read here rather than a
    /// database being asked for an opinion.
    #[test]
    fn every_variant_is_one_the_check_constraint_allows() {
        let schema = include_str!("../../migrations/2026-08-05-000001_initial_schema/up.sql");

        let cases: [(&str, Vec<&'static str>); 3] = [
            (
                "accounts_kind_known",
                KINDS.iter().map(AccountKind::as_str).collect(),
            ),
            (
                "accounts_availability_known",
                AccountAvailability::ORDERED
                    .iter()
                    .map(AccountAvailability::as_str)
                    .collect(),
            ),
            (
                "accounts_risk_known",
                RISKS.iter().map(AccountRisk::as_str).collect(),
            ),
        ];

        for (constraint, variants) in cases {
            let rest = schema
                .split_once(constraint)
                .unwrap_or_else(|| panic!("{constraint} is gone from the migration"))
                .1;
            let allowed = &rest[..rest.find(')').expect("the CHECK list closes")];

            for variant in &variants {
                assert!(
                    allowed.contains(&format!("'{variant}'")),
                    "{constraint} does not allow '{variant}'"
                );
            }

            // and nothing the enum cannot name, which is the direction a widened CHECK breaks
            assert_eq!(
                allowed.matches('\'').count() / 2,
                variants.len(),
                "{constraint} and the enum list different numbers of values"
            );
        }
    }

    /// `from_db` is the only thing standing between a row written by hand and a variant Rust
    /// does not know about.
    #[test]
    fn the_text_mapping_round_trips() {
        for kind in KINDS {
            assert_eq!(
                AccountKind::from_db(kind.as_str()).map(|k| k.as_str()),
                Ok(kind.as_str())
            );
        }

        assert!(AccountKind::from_db("crypto").is_err());
    }
}
