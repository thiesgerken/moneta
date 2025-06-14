use chrono::{DateTime, Utc};
use juniper::{GraphQLInputObject, GraphQLObject};
use serde::{Deserialize, Serialize};

use super::{
    expense::{
        ExpenseCategorizationInput, ExpensePositionInput, ExpenseReceiptInput,
        ExpenseTransactionInput,
    },
    ExpenseCategorization, ExpensePosition, ExpenseReceipt, ObjectIdWrapper,
};

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[serde(rename_all = "camelCase")]
#[graphql(name = "Expense")]
pub struct RenderedExpense {
    #[serde(rename = "_id", default)]
    pub id: ObjectIdWrapper,

    /// id of the expense that was rendered
    pub upstream_id: ObjectIdWrapper,

    /// user for whom this expense was rendered
    #[serde(default)]
    pub user_id: ObjectIdWrapper,

    pub title: String,
    pub store: String,
    pub description: String,
    pub comments: String,

    pub reference_amount: Option<i32>,

    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub booking_start: DateTime<Utc>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub booking_end: DateTime<Utc>,

    /// set to true if this expense is only a stanza for a recurring expense; this means it will not be counted in any analysis
    pub is_template: bool,
    /// to track expenses that should be looked at again when preparing tax forms
    pub is_tax_relevant: bool,
    /// needs to be looked at by a user first (missing info, low confidence, ...)
    pub is_unchecked: bool,
    /// if we anticipate more transactions to appear before this can be completed; e.g. payback when a order hasn't arrived
    pub is_incomplete: bool,
    /// if this expense should not be considered for monthly spending and so on
    pub is_extraordinary: bool,

    pub receipts: Vec<ExpenseReceipt>,
    pub categories: Vec<ExpenseCategorization>,
    pub transactions: Vec<RenderedExpenseTransaction>,
    pub positions: Vec<ExpensePosition>,

    // how much the total wealth of the user this was rendered for changed by this expense
    // (sum of the effective_amounts of all the rendered transactions of this rendered expense)
    pub effective_amount: f64,

    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[serde(rename_all = "camelCase")]
#[graphql(name = "ExpenseTransaction")]
pub struct RenderedExpenseTransaction {
    pub account_id: ObjectIdWrapper,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub date: DateTime<Utc>,

    pub amount: Option<i32>,
    pub fraction: Option<f64>,

    pub comments: String,
    pub statement: String,

    pub calculated_amount: f64,

    // how much the total wealth of the user this was rendered for changed by this transaction
    // (takes into account if the account belongs to the user and looks at the ownership fractions)
    pub effective_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[serde(rename_all = "camelCase")]
#[graphql(name = "Balance")]
pub struct RenderedBalance {
    #[serde(rename = "_id", default)]
    pub id: ObjectIdWrapper,

    /// id of the balance that was rendered
    pub upstream_id: ObjectIdWrapper,

    /// user for whom this balance was rendered
    #[serde(default)]
    pub user_id: ObjectIdWrapper,

    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub date: DateTime<Utc>, // includes expenses with date < this value (i.e. what happened the same instant is not included)

    pub account_id: ObjectIdWrapper,
    pub amount: i32, // cents
    pub comments: String,

    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLInputObject)]
#[graphql(name = "BalanceInput")]
#[serde(rename_all = "camelCase")]
pub struct RenderedBalanceInput {
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub date: DateTime<Utc>, // includes expenses with date < this value (i.e. what happened the same instant is not included)

    pub account_id: ObjectIdWrapper,
    pub amount: i32, // cents
    pub comments: String,
}

pub type RenderedBalanceUpdateInput = RenderedBalanceInput;
pub type RenderedBalanceCreateInput = RenderedBalanceInput;

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLInputObject)]
#[graphql(name = "ExpenseInput")]
#[serde(rename_all = "camelCase")]
pub struct RenderedExpenseInput {
    pub title: String,
    pub store: String,
    pub description: String,
    pub comments: String,

    pub reference_amount: Option<i32>,

    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub booking_start: DateTime<Utc>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub booking_end: DateTime<Utc>,

    /// set to true if this expense is only a stanza for a recurring expense; this means it will not be counted in any analysis
    pub is_template: bool,
    /// to track expenses that should be looked at again when preparing tax forms
    pub is_tax_relevant: bool,
    /// needs to be looked at by a user first (missing info, low confidence, ...)
    pub is_unchecked: bool,
    /// if we anticipate more transactions to appear before this can be completed; e.g. payback when a order hasn't arrived
    pub is_incomplete: bool,
    /// if this expense should not be considered for monthly spending and so on
    pub is_extraordinary: bool,

    pub receipts: Vec<ExpenseReceiptInput>,
    pub categories: Vec<ExpenseCategorizationInput>,
    pub transactions: Vec<ExpenseTransactionInput>,
    pub positions: Vec<ExpensePositionInput>,
}

pub type RenderedExpenseUpdateInput = RenderedExpenseInput;
pub type RenderedExpenseCreateInput = RenderedExpenseInput;
