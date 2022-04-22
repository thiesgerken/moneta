use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
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
    #[serde(rename = "_id")]
    pub id: ObjectId,

    pub user_id: ObjectId,
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
    #[serde(with = "optional_chrono_datetime_as_bson_datetime")]
    pub closed: Option<DateTime<Utc>>,

    // how to add balances of this account to the total balance, must be >0 and <=1
    pub ownership_fraction: f32,

    // of course this needs to be mirrored in the other account
    pub synchronization: Option<AccountSynchronization>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSynchronization {
    pub account_id: ObjectId, // id of the other account
    pub invert: bool, // whether to count income in the original account as an expense in the synchronized account (and vice versa) or not
}
pub mod optional_chrono_datetime_as_bson_datetime {
    use chrono::Utc;
    use mongodb::bson;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::result::Result;

    /// Deserializes a [`Option<chrono::DateTime>`] from a [`Option<crate::DateTime>`].
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<chrono::DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        if let Ok(datetime) = bson::DateTime::deserialize(deserializer) {
            Ok(Some(datetime.to_chrono()))
        } else {
            // NOTE: this implementation simply returns `None` whenever the value present
            // cannot be parsed as a datetime; this might not be the best behavior
            Ok(None)
        }
    }

    /// Serializes a [`Option<chrono::DateTime>`] as a [`Option<crate::DateTime>`].
    pub fn serialize<S: Serializer>(
        val: &Option<chrono::DateTime<Utc>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        if let Some(val) = val {
            let datetime = bson::DateTime::from_chrono(val.to_owned());
            datetime.serialize(serializer)
        } else {
            bson::Bson::Null.serialize(serializer)
        }
    }
}
