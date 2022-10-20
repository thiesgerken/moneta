mod account;
mod expense;
pub mod rendering;

use std::ops::Deref;

pub use account::{
    Account, AccountAvailability, AccountCreateInput, AccountInput, AccountKind, AccountRisk,
    AccountSynchronization, AccountUpdateInput,
};
pub use expense::{
    Expense, ExpenseCategorization, ExpenseCategorizationInput, ExpensePosition,
    ExpensePositionInput, ExpenseReceipt, ExpenseReceiptInput, ExpenseTransaction,
    ExpenseTransactionInput,
};

use chrono::{DateTime, Utc};
use juniper::{
    graphql_scalar,
    parser::{ScalarToken, Token},
    GraphQLInputObject, GraphQLObject, InputValue, ParseError, ParseScalarResult, Value,
};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ObjectIdWrapper(pub ObjectId);

#[graphql_scalar(name = "ObjectId")]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(rename = "_id", default)]
    pub id: ObjectIdWrapper,

    pub name: String,
    pub full_name: String,
    pub password_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
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
    pub monthly_limit: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLInputObject)]
#[serde(rename_all = "camelCase")]
pub struct CategoryInput {
    pub name: String,
    pub description: String,
    pub color: Option<String>,
    pub parent: Option<ObjectIdWrapper>,

    pub monthly_limit: Option<f64>,
}

impl CategoryInput {
    pub fn upgrade(self, id: ObjectIdWrapper) -> Category {
        Category {
            id,
            name: self.name,
            description: self.description,
            color: self.color,
            parent: self.parent,
            monthly_limit: self.monthly_limit,
        }
    }
}

pub type CategoryUpdateInput = CategoryInput;
pub type CategoryCreateInput = CategoryInput;

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[serde(rename_all = "camelCase")]
pub struct Balance {
    #[serde(rename = "_id", default)]
    pub id: ObjectIdWrapper,

    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub date: DateTime<Utc>, // includes expenses with date < this value (i.e. what happened the same instant is not included)

    pub account_id: ObjectIdWrapper,
    pub amount: i32, // cents
    pub comments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseChangelogEntry {
    pub target_id: ObjectIdWrapper,
    pub user_id: Option<ObjectIdWrapper>,

    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub date: DateTime<Utc>,

    /// name of the program that was used to make the change (e.g. web app, CLI, ...)
    pub tool: String,

    /// whether this action was performed by a human or an automatic system
    pub automatic: bool,
    /// if this action was performed by an automated (AI) system, an indication (between 0 and 1) about how sure this system was.
    /// value should be ignored if the action was performed by a human
    pub confidence: f64,

    /// how the entry looked before and after the change.
    /// either one of these (but not both
    /// representing the creation or deletion of the entry.
    pub old_state: Option<Expense>,
    pub new_state: Option<Expense>,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[serde(rename_all = "camelCase")]
pub struct BalanceChangelogEntry {
    pub target_id: ObjectIdWrapper,
    pub user_id: Option<ObjectIdWrapper>,

    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub date: DateTime<Utc>,

    /// name of the program that was used to make the change (e.g. web app, CLI, ...)
    pub tool: String,

    /// whether this action was performed by a human or an automatic system
    pub automatic: bool,
    /// if this action was performed by an automated (AI) system, an indication (between 0 and 1) about how sure this system was.
    /// value should be ignored if the action was performed by a human
    pub confidence: f64,

    /// how the entry looked before and after the change.
    /// either one of these (but not both
    /// representing the creation or deletion of the entry.
    pub old_state: Option<Balance>,
    pub new_state: Option<Balance>,
}

// pub type ExpenseChangelogEntry = ChangelogEntry<Expense>;
// pub type BalanceChangelogEntry = ChangelogEntry<Balance>;
