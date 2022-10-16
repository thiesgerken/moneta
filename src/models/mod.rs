mod account;
mod expense;
pub mod rendering;

use std::ops::Deref;

pub use account::{Account, AccountAvailability, AccountKind, AccountRisk, AccountSynchronization};
pub use expense::{
    Expense, ExpenseCategorization, ExpensePosition, ExpenseReceipt, ExpenseTransaction,
};

use chrono::{DateTime, Utc};
use juniper::{GraphQLObject, graphql_scalar, Value, InputValue, ParseScalarResult, ParseError, parser::{ScalarToken, Token}};
use mongodb::bson::oid::ObjectId;
use rocket_okapi::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ObjectIdWrapper(pub ObjectId);

#[graphql_scalar(name = "OID")]
impl<S> GraphQLScalar for ObjectIdWrapper
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        Value::scalar(self.0.to_hex())
    }

    fn from_input_value(v: &InputValue) -> Option<ObjectIdWrapper> {
        match *v {
            InputValue::Scalar(ref s) => s
                .as_string()
                .and_then(|s| ObjectId::parse_str(s).ok())
                .map(ObjectIdWrapper),
            _ => None,
        }
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        match value {
            ScalarToken::String(value) | ScalarToken::Int(value) => Ok(S::from(value.to_owned())),
            _ => Err(ParseError::UnexpectedToken(Token::Scalar(value))),
        }
    }
}

impl Deref for ObjectIdWrapper {
    type Target = ObjectId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ObjectIdWrapper> for ObjectId {
    fn from(o: ObjectIdWrapper) -> Self {
        o.0
    }
}

impl From<ObjectId> for ObjectIdWrapper {
    fn from(o: ObjectId) -> Self {
        ObjectIdWrapper(o)
    }
}

impl JsonSchema for ObjectIdWrapper {
    fn schema_name() -> String {
        "ObjectId".to_owned()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        #[derive(JsonSchema)]
        struct ExtendedObjectId {
            #[serde(rename = "$oid")]
            _id: String,
        }

        ExtendedObjectId::json_schema(gen)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(rename = "_id", default)]
    pub id: ObjectIdWrapper,

    pub name: String,
    pub full_name: String,
    pub password_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    #[serde(rename = "_id", default)]
    pub id: ObjectIdWrapper,

    pub name: String,
    pub description: String,
    pub color: Option<String>,
    pub parent: Option<ObjectIdWrapper>,

    /// desired max spending in this category per month.
    /// In the future this could become a list of limits and timespans
    /// (to accommodate for varying spending limits over time without changing past data)
    pub monthly_limit: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Balance {
    #[serde(rename = "_id", default)]
    pub id: ObjectIdWrapper,

    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    #[schemars(with = "DateTime<Utc>")]
    pub date: DateTime<Utc>, // includes expenses with date < this value (i.e. what happened the same instant is not included)

    pub account_id: ObjectIdWrapper,
    pub amount: i64, // cents
    pub comments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangelogEntry<T> {
    pub target_id: ObjectIdWrapper,
    pub user_id: Option<ObjectIdWrapper>,

    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    #[schemars(with = "DateTime<Utc>")]
    pub date: DateTime<Utc>,

    /// name of the program that was used to make the change (e.g. web app, CLI, ...)
    pub tool: String,

    /// whether this action was performed by a human or an automatic system
    pub automatic: bool,
    /// if this action was performed by an automated (AI) system, an indication (between 0 and 1) about how sure this system was.
    /// value should be ignored if the action was performed by a human
    pub confidence: f32,

    /// how the entry looked before and after the change.
    /// either one of these (but not both
    /// representing the creation or deletion of the entry.
    pub old_state: Option<T>,
    pub new_state: Option<T>,
}

pub type ExpenseChangelogEntry = ChangelogEntry<Expense>;
pub type BalanceChangelogEntry = ChangelogEntry<Balance>;
