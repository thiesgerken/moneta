use crate::models::*;

use log::error;
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
    pub total_amount: i64,
    pub calculated_amounts: Vec<i64>, // for each transaction
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

pub fn calculate_total_amount(
    uid: i32,
    info: &Expense,
    transactions: &[(ExpenseTransaction, Account, Option<AccountSynchronization>)],
) -> (i64, Vec<i64>) {
    let ts_with_amount = transactions
        .iter()
        .filter_map(|(t, _, _)| t.amount)
        .collect::<Vec<_>>();
    if ts_with_amount.is_empty() {
        error!(
            "expense {} has no transactions with a fixed amount! pretending that its 0.",
            info.id
        );
    } else if ts_with_amount.len() > 1 && ts_with_amount.len() < transactions.len() {
        error!(
        "expense {} has multiple transactions with a fixed amount, and also fractional transactions. This is unsupported behavior!",
        info.id
    );
    }
    let sum_of_amounts = ts_with_amount.iter().sum::<i64>();
    let mut total_amount: i64 = 0;

    // if there is an account syncing, then the synced account has to belong to the user anyway, no need to check uids in that case
    total_amount += transactions
        .iter()
        .filter(|(_, a, acs)| acs.is_some() || a.user_id == uid)
        .filter_map(|(t, _, _)| t.amount)
        .sum::<i64>();
    total_amount += transactions
        .iter()
        .filter(|(_, a, acs)| acs.is_some() || a.user_id == uid)
        .filter_map(|(t, _, _)| t.fraction.map(|x| (x * sum_of_amounts as f64) as i64))
        .sum::<i64>();

    let calculated_amounts = transactions
        .iter()
        .map(|(t, _, _)| {
            t.amount
                .unwrap_or_else(|| (t.fraction.unwrap_or(0.0) * sum_of_amounts as f64) as i64)
        })
        .collect::<Vec<_>>();

    (total_amount, calculated_amounts)
}

impl RenderedExpense {
    // it is very important that the transactions only contain account synchronizations if the account in the transaction does not belong to the user!
    pub fn render(
        uid: i32,
        info: Expense,
        transactions: Vec<(ExpenseTransaction, Account, Option<AccountSynchronization>)>,
        categories: Vec<(ExpenseCategory, Option<CategoryReplacement>)>,
        receipts: Vec<ExpenseReceipt>,
        events: Vec<ExpenseEvent>,
    ) -> RenderedExpense {
        let transactions = transactions
            .into_iter()
            .map(|(mut t, a, acs)| {
                if let Some(acs) = &acs {
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
                (t, a, acs)
            })
            .collect::<Vec<_>>();

        let (total_amount, calculated_amounts) = calculate_total_amount(uid, &info, &transactions);

        let categories = categories
            .into_iter()
            .map(|(mut c, repl)| {
                if let Some(repl) = repl {
                    c.category_id = repl.replacement;
                }
                c
            })
            .collect::<Vec<ExpenseCategory>>();

        let transactions = transactions
            .into_iter()
            .map(|(t, _, _)| t)
            .collect::<Vec<_>>();

        RenderedExpense {
            info,
            categories,
            events,
            transactions,
            receipts,
            total_amount,
            calculated_amounts,
        }
    }

    // assumes that the input vectors are sorted by expense_id!
    pub fn filter_and_render(
        uid: i32,
        info: Expense,
        transactions: &[(ExpenseTransaction, Account, Option<AccountSynchronization>)],
        categories: &[(ExpenseCategory, Option<CategoryReplacement>)],
        receipts: &[ExpenseReceipt],
        events: &[ExpenseEvent],
    ) -> RenderedExpense {
        let transactions = transactions
            .iter()
            // .filter(|(t, _)| t.expense_id == info.id)
            .skip_while(|(t, _, _)| t.expense_id != info.id)
            .take_while(|(t, _, _)| t.expense_id == info.id)
            .map(|(t, a, acs)| {
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
                (t, a.clone(), acs.clone())
            })
            .collect::<Vec<_>>();

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

        let (total_amount, calculated_amounts) = calculate_total_amount(uid, &info, &transactions);
        let transactions = transactions
            .into_iter()
            .map(|(t, _, _)| t)
            .collect::<Vec<_>>();

        RenderedExpense {
            info,
            categories,
            events,
            transactions,
            receipts,
            total_amount,
            calculated_amounts,
        }
    }
}
