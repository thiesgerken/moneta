mod account;
mod expense;
mod json;
pub mod rendering;
mod rules;
mod text_enum;

pub use json::Json;
pub(crate) use text_enum::text_enum;

pub use account::{
    Account, AccountAvailability, AccountCreateInput, AccountInput, AccountKind, AccountRisk,
    AccountSynchronization, AccountUpdateInput,
};
pub use expense::{
    Expense, ExpenseCategorization, ExpenseCategorizationInput, ExpenseCategorizationRow,
    ExpensePosition, ExpensePositionInput, ExpenseReceipt, ExpenseReceiptInput, ExpenseRow,
    ExpenseTransaction, ExpenseTransactionInput, ExpenseTransactionRow,
};
pub use rules::{
    AutomationRule, AutomationRuleInput, ExpenseDelivery, PeriodicInterval, PeriodicRule,
    PeriodicRuleInput, SeenDelivery,
};

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use juniper::{GraphQLInputObject, GraphQLObject};
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::users, check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: Uuid,

    pub name: String,
    pub full_name: String,
    pub password_hash: String,
}

#[derive(Debug, Clone, GraphQLObject, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::categories, check_for_backend(diesel::pg::Pg))]
pub struct Category {
    pub id: Uuid,

    pub name: String,
    pub description: String,
    pub color: Option<String>,
    pub parent_id: Option<Uuid>,

    /// desired max spending in this category per month, in cents.
    /// In the future this could become a list of limits and timespans
    /// (to accommodate for varying spending limits over time without changing past data)
    pub monthly_limit: Option<i32>,
}

#[derive(Debug, Clone, GraphQLInputObject)]
pub struct CategoryInput {
    pub name: String,
    pub description: String,
    pub color: Option<String>,
    pub parent_id: Option<Uuid>,

    pub monthly_limit: Option<i32>,
}

impl CategoryInput {
    pub fn upgrade(self, id: Uuid) -> Category {
        Category {
            id,
            name: self.name,
            description: self.description,
            color: self.color,
            parent_id: self.parent_id,
            monthly_limit: self.monthly_limit,
        }
    }
}

pub type CategoryUpdateInput = CategoryInput;
pub type CategoryCreateInput = CategoryInput;

#[derive(Debug, Clone, GraphQLObject, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::balances, check_for_backend(diesel::pg::Pg))]
pub struct Balance {
    pub id: Uuid,

    pub account_id: Uuid,
    /// includes expenses with date < this value (i.e. what happened the same instant is not
    /// included)
    pub date: DateTime<Utc>,
    pub amount: i32, // cents
    pub comments: String,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::expense_changes, check_for_backend(diesel::pg::Pg))]
pub struct ExpenseChangelogEntry {
    pub id: Uuid,

    /// not a foreign key: a change outlives the expense it describes
    pub target_id: Uuid,
    pub user_id: Option<Uuid>,

    pub date: DateTime<Utc>,

    /// name of the program that was used to make the change (e.g. web app, CLI, ...)
    pub tool: Option<String>,

    /// whether this action was performed by a human or an automatic system
    pub automatic: bool,
    /// if this action was performed by an automated (AI) system, an indication (between 0 and 1)
    /// about how sure this system was. Value should be ignored if a human made the change.
    pub confidence: f64,

    /// How the entry looked before and after the change; one of them is absent when the change
    /// created or deleted the expense.
    ///
    /// Kept as whole documents rather than normalised, so that a later schema change cannot
    /// retroactively alter what the history says happened.
    pub old_state: Option<Json<Expense>>,
    pub new_state: Option<Json<Expense>>,
}
