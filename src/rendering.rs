use crate::models::*;

use serde::{Deserialize, Serialize};

// The contents of these "Rendered" entities may have been altered to be viewed by the user that requests them

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedAccount {
    pub info: Account,
    pub synchronization: Option<AccountSynchronization>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedExpense {
    pub info: Expense,
    pub events: Vec<ExpenseEvent>,
    pub receipts: Vec<ExpenseReceipt>,
    pub categories: Vec<ExpenseCategory>,
    pub transactions: Vec<ExpenseTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedCategory {
    pub info: Category,
    pub replaces: Vec<i32>,
}

pub type RenderedBalance = Balance;

impl RenderedBalance {
    pub fn render(
        mut balance: Balance,
        account_sync: Option<AccountSynchronization>,
    ) -> RenderedBalance {
        if let Some(sync) = account_sync {
            balance.account_id = if sync.account1 == balance.account_id {
                sync.account2
            } else {
                sync.account1
            };
            balance.amount *= if sync.invert { -1 } else { 1 };
        }

        balance
    }
}

impl RenderedCategory {
    pub fn render(category: Category, replaces: &[CategoryReplacement]) -> RenderedCategory {
        RenderedCategory {
            replaces: replaces
                .iter()
                .filter(|c| c.replacement == category.id)
                .map(|c| c.original)
                .collect(),
            info: category,
        }
    }
}

impl RenderedExpense {
    pub fn render(
        info: Expense,
        transactions: Vec<(ExpenseTransaction, Option<AccountSynchronization>)>,
        categories: Vec<(ExpenseCategory, Option<CategoryReplacement>)>,
        receipts: Vec<ExpenseReceipt>,
        events: Vec<ExpenseEvent>,
    ) -> RenderedExpense {
        let transactions = transactions
            .into_iter()
            .map(|(mut t, acs)| {
                if let Some(acs) = acs {
                    if acs.invert {
                        t.amount = t.amount.map(|y| -y);
                        t.fraction = t.fraction.map(|y| -y);
                    }

                    t.account_id = if acs.account1 == t.account_id {
                        acs.account2
                    } else {
                        acs.account1
                    };
                }
                t
            })
            .collect::<Vec<ExpenseTransaction>>();

        let categories = categories
            .into_iter()
            .map(|(mut c, repl)| {
                if let Some(repl) = repl {
                    c.category_id = repl.replacement;
                }
                c
            })
            .collect::<Vec<ExpenseCategory>>();

        RenderedExpense {
            info,
            categories,
            events,
            transactions,
            receipts,
        }
    }

    // assumes that the input vectors are sorted by expense_id!
    pub fn filter_and_render(
        info: Expense,
        transactions: &[(ExpenseTransaction, Option<AccountSynchronization>)],
        categories: &[(ExpenseCategory, Option<CategoryReplacement>)],
        receipts: &[ExpenseReceipt],
        events: &[ExpenseEvent],
    ) -> RenderedExpense {
        let transactions = transactions
            .iter()
            // .filter(|(t, _)| t.expense_id == info.id)
            .skip_while(|(t, _)| t.expense_id != info.id)
            .take_while(|(t, _)| t.expense_id == info.id)
            .map(|(t, acs)| {
                let mut t = t.clone();

                if let Some(acs) = acs {
                    if acs.invert {
                        t.amount = t.amount.map(|y| -y);
                        t.fraction = t.fraction.map(|y| -y);
                    }

                    t.account_id = if acs.account1 == t.account_id {
                        acs.account2
                    } else {
                        acs.account1
                    };
                }
                t
            })
            .collect::<Vec<ExpenseTransaction>>();

        let categories = categories
            .iter()
            // .filter(|(c, _)| c.expense_id == info.id)
            .skip_while(|(t, _)| t.expense_id != info.id)
            .take_while(|(t, _)| t.expense_id == info.id)
            .map(|(c, repl)| {
                let mut c = c.clone();

                if let Some(repl) = repl {
                    c.category_id = repl.replacement;
                }
                c
            })
            .collect::<Vec<ExpenseCategory>>();

        let events = events
            .iter()
            // .filter(|e| e.expense_id == info.id)
            .skip_while(|t| t.expense_id != info.id)
            .take_while(|t| t.expense_id == info.id)
            .cloned()
            .collect();

        let receipts = receipts
            .iter()
            // .filter(|r| r.expense_id == info.id)
            .skip_while(|t| t.expense_id != info.id)
            .take_while(|t| t.expense_id == info.id)
            .cloned()
            .collect();

        RenderedExpense {
            info,
            categories,
            events,
            transactions,
            receipts,
        }
    }
}
