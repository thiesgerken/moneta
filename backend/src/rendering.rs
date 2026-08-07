//! Converting what the API accepts into what the tables store.
//!
//! The other direction — turning upstream rows into the per-user view of them — used to live
//! here as `render`/`render_for_user`, which wrote one document per involved user into
//! `expenses_rendered` and `balances_rendered`. Rendering is a pure function of (expense,
//! accounts), so it is the `*_rendered` views now, and the rerender step (a `delete_many`
//! followed by an `insert_many`, with no transaction around it) is gone with it. The cases it
//! used to be tested for are covered in `tests/sql/rendered_views.sql`.

use chrono::Utc;
use uuid::Uuid;

use crate::models::{
    Balance, Expense, ExpenseCategorizationInput, ExpensePositionInput, ExpenseReceiptInput,
    ExpenseTransaction, ExpenseTransactionInput,
    rendering::{RenderedBalanceInput, RenderedExpenseInput},
};

impl Balance {
    pub fn from_rendered(balance: RenderedBalanceInput) -> Self {
        let now = Utc::now();

        Self {
            id: Uuid::now_v7(),
            date: balance.date,
            account_id: balance.account_id,
            amount: balance.amount,
            comments: balance.comments,
            created_at: now,
            updated_at: now,
        }
    }
}

impl ExpenseTransaction {
    pub fn from_rendered(transaction: ExpenseTransactionInput) -> Self {
        transaction.upgrade()
    }
}

impl Expense {
    pub fn from_rendered(expense: RenderedExpenseInput) -> Self {
        let now = Utc::now();

        Self {
            id: Uuid::now_v7(),
            title: expense.title,
            store: expense.store,
            description: expense.description,
            comments: expense.comments,
            accrual_start: expense.accrual_start,
            accrual_end: expense.accrual_end,
            reference_amount: expense.reference_amount,
            is_template: expense.is_template,
            is_tax_relevant: expense.is_tax_relevant,
            is_unchecked: expense.is_unchecked,
            is_incomplete: expense.is_incomplete,
            is_extraordinary: expense.is_extraordinary,
            receipts: expense
                .receipts
                .into_iter()
                .map(ExpenseReceiptInput::upgrade)
                .collect(),
            categories: expense
                .categories
                .into_iter()
                .map(ExpenseCategorizationInput::upgrade)
                .collect(),
            transactions: expense
                .transactions
                .into_iter()
                .map(ExpenseTransaction::from_rendered)
                .collect(),
            positions: expense
                .positions
                .into_iter()
                .map(ExpensePositionInput::upgrade)
                .collect(),
            created_at: now,
            updated_at: now,
        }
    }
}
