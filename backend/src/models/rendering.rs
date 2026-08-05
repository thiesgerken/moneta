//! The per-user view of the upstream data.
//!
//! These are read out of the `*_rendered` views rather than stored, so none of them is
//! `Insertable`. The nested lists arrive as jsonb the view aggregated, which is why they are
//! wrapped in [`Json`] rather than being separate queries.

use uuid::Uuid;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use juniper::{GraphQLInputObject, GraphQLObject};
use serde::{Deserialize, Serialize};

use super::{
    ExpenseCategorization, ExpensePosition, ExpenseReceipt, Json,
    expense::{
        ExpenseCategorizationInput, ExpensePositionInput, ExpenseReceiptInput,
        ExpenseTransactionInput,
    },
};

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = crate::schema_views::expenses_rendered)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RenderedExpense {
    /// A reader only ever sees their own rendering, so the upstream id identifies it for
    /// them; the view returns the same value for both.
    pub id: Uuid,
    pub upstream_id: Uuid,

    /// user this was rendered for
    pub user_id: Uuid,

    pub title: String,
    pub store: String,
    pub description: String,
    pub comments: String,

    pub reference_amount: Option<i32>,

    pub booking_start: DateTime<Utc>,
    pub booking_end: DateTime<Utc>,

    pub is_template: bool,
    pub is_tax_relevant: bool,
    pub is_unchecked: bool,
    pub is_incomplete: bool,
    pub is_extraordinary: bool,

    pub receipts: Json<Vec<ExpenseReceipt>>,
    pub positions: Json<Vec<ExpensePosition>>,
    pub categories: Json<Vec<ExpenseCategorization>>,
    pub transactions: Json<Vec<RenderedExpenseTransaction>>,

    /// how much the total wealth of the user this was rendered for changed by this expense
    /// (sum of the effective_amounts of all its rendered transactions)
    pub effective_amount: f64,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Comes out of the aggregated jsonb rather than from a row, so the field names have to match
/// what `expenses_rendered` builds.
#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[serde(rename_all = "camelCase")]
#[graphql(name = "ExpenseTransaction")]
pub struct RenderedExpenseTransaction {
    pub account_id: Uuid,
    pub date: DateTime<Utc>,

    pub amount: Option<i32>,
    pub fraction: Option<f64>,

    pub comments: String,
    pub statement: String,

    pub calculated_amount: f64,

    /// how much the total wealth of the user this was rendered for changed by this
    /// transaction (accounts for whether the account belongs to them, and for ownership
    /// fractions)
    pub effective_amount: f64,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = crate::schema_views::balances_rendered)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RenderedBalance {
    pub id: Uuid,
    pub upstream_id: Uuid,
    pub user_id: Uuid,

    /// includes expenses with date < this value (i.e. what happened the same instant is not
    /// included)
    pub date: DateTime<Utc>,

    pub account_id: Uuid,
    pub amount: i32, // cents
    pub comments: String,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, GraphQLInputObject)]
#[graphql(name = "BalanceInput")]
pub struct RenderedBalanceInput {
    /// includes expenses with date < this value
    pub date: DateTime<Utc>,

    pub account_id: Uuid,
    pub amount: i32, // cents
    pub comments: String,
}

pub type RenderedBalanceUpdateInput = RenderedBalanceInput;
pub type RenderedBalanceCreateInput = RenderedBalanceInput;

#[derive(Debug, Clone, GraphQLInputObject)]
#[graphql(name = "ExpenseInput")]
pub struct RenderedExpenseInput {
    pub title: String,
    pub store: String,
    pub description: String,
    pub comments: String,

    pub reference_amount: Option<i32>,

    pub booking_start: DateTime<Utc>,
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
