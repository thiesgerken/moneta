use crate::models::{rendering::RenderedBalance, Account, Balance};
use mongodb::{
    bson::{doc, oid::ObjectId},
    results::DeleteResult,
    Database,
};
use std::error::Error;

impl Balance {
    pub fn from_rendered(balance: RenderedBalance) -> Self {
        Self {
            id: ObjectId::new().into(),
            date: balance.date,
            account_id: balance.account_id,
            amount: balance.amount,
            comments: balance.comments,
        }
    }

    pub fn render(&self, accounts: &[Account]) -> Result<Vec<RenderedBalance>, Box<dyn Error>> {
        let main_account = accounts
            .iter()
            .find(|a| a.id == self.account_id)
            .ok_or_else(|| format!("Cannot find main account of {:?}", &self))?;

        let mut res = vec![RenderedBalance {
            id: ObjectId::new().into(),
            upstream_id: self.id.clone(),
            user_id: main_account.user_id.clone(),
            date: self.date,
            account_id: main_account.id.clone(),
            amount: self.amount,
            comments: self.comments.clone(),
        }];

        if let Some(sync) = &main_account.synchronization {
            let other_account = accounts
                .iter()
                .find(|a| a.id == sync.account_id)
                .ok_or_else(|| {
                    format!("Cannot find synchronized account of {:?}", &main_account)
                })?;

            res.push(RenderedBalance {
                id: ObjectId::new().into(),
                upstream_id: self.id.clone(),
                user_id: other_account.user_id.clone(),
                date: self.date,
                account_id: other_account.id.clone(),
                amount: self.amount * if sync.invert { -1 } else { 1 },
                comments: self.comments.clone(),
            });
        }

        Ok(res)
    }

    pub fn apply_changes(&mut self, changes: &RenderedBalance) {
        self.account_id = changes.account_id.clone();
        self.amount = changes.amount;
    }

    pub async fn remove_rendered(&self, db: &Database) -> Result<DeleteResult, Box<dyn Error>> {
        let rendered_balances_collection = db.collection::<RenderedBalance>("balances_rendered");
        let r = rendered_balances_collection
            .delete_many(doc!("upstream_id": *self.id.clone()), None)
            .await?;

        info!(
            "deleted {} rendered balances of balance {:?}",
            r.deleted_count, &self.id
        );
        Ok(r)
    }
}

// pub fn calculate_total_amount(
//     uid: i32,
//     info: &Expense,
//     transactions: &[(ExpenseTransaction, Account, Option<AccountSynchronization>)],
// ) -> (i64, Vec<i64>) {
//     let ts_with_amount = transactions
//         .iter()
//         .filter_map(|(t, _, _)| t.amount)
//         .collect::<Vec<_>>();
//     if ts_with_amount.is_empty() {
//         error!(
//             "expense {} has no transactions with a fixed amount! pretending that its 0.",
//             info.id
//         );
//     } else if ts_with_amount.len() > 1 && ts_with_amount.len() < transactions.len() {
//         error!(
//         "expense {} has multiple transactions with a fixed amount, and also fractional transactions. This is unsupported behavior!",
//         info.id
//     );
//     }
//     let sum_of_amounts = ts_with_amount.iter().sum::<i64>();
//     let mut total_amount: i64 = 0;

//     // if there is an account syncing, then the synced account has to belong to the user anyway, no need to check uids in that case
//     total_amount += transactions
//         .iter()
//         .filter(|(_, a, acs)| acs.is_some() || a.user_id == uid)
//         .filter_map(|(t, _, _)| t.amount)
//         .sum::<i64>();
//     total_amount += transactions
//         .iter()
//         .filter(|(_, a, acs)| acs.is_some() || a.user_id == uid)
//         .filter_map(|(t, _, _)| t.fraction.map(|x| (x * sum_of_amounts as f64) as i64))
//         .sum::<i64>();

//     let calculated_amounts = transactions
//         .iter()
//         .map(|(t, _, _)| {
//             t.amount
//                 .unwrap_or_else(|| (t.fraction.unwrap_or(0.0) * sum_of_amounts as f64) as i64)
//         })
//         .collect::<Vec<_>>();

//     (total_amount, calculated_amounts)
// }

// impl RenderedExpense {
//     // it is very important that the transactions only contain account synchronizations if the account in the transaction does not belong to the user!
//     pub fn render(
//         uid: i32,
//         info: Expense,
//         transactions: Vec<(ExpenseTransaction, Account, Option<AccountSynchronization>)>,
//         categories: Vec<(ExpenseCategory, Option<CategoryReplacement>)>,
//         receipts: Vec<ExpenseReceipt>,
//         positions: Vec<ExpensePosition>,
//         events: Vec<ExpenseEvent>,
//         group: Option<ExpenseGroup>,
//     ) -> RenderedExpense {
//         assert!(info.group_id.is_none() == group.is_none());

//         let transactions = transactions
//             .into_iter()
//             .map(|(mut t, a, acs)| {
//                 if let Some(acs) = &acs {
//                     if acs.invert {
//                         t.amount = t.amount.map(|y| -y);
//                         t.fraction = t.fraction.map(|y| -y);
//                     }

//                     t.account_id = if acs.account1 == t.account_id {
//                         acs.account2
//                     } else {
//                         acs.account1
//                     };
//                 }
//                 (t, a, acs)
//             })
//             .collect::<Vec<_>>();

//         let (total_amount, calculated_amounts) = calculate_total_amount(uid, &info, &transactions);

//         let categories = categories
//             .into_iter()
//             .map(|(mut c, repl)| {
//                 if let Some(repl) = repl {
//                     c.category_id = repl.replacement;
//                 }
//                 c
//             })
//             .collect::<Vec<ExpenseCategory>>();

//         let transactions = transactions
//             .into_iter()
//             .map(|(t, _, _)| t)
//             .collect::<Vec<_>>();

//         RenderedExpense {
//             info,
//             categories,
//             events,
//             transactions,
//             positions,
//             receipts,
//             group,
//             total_amount,
//             calculated_amounts,
//         }
//     }

//     // assumes that the input vectors are sorted by expense_id!
//     pub fn filter_and_render(
//         uid: i32,
//         info: Expense,
//         transactions: &[(ExpenseTransaction, Account, Option<AccountSynchronization>)],
//         categories: &[(ExpenseCategory, Option<CategoryReplacement>)],
//         positions: &[ExpensePosition],
//         receipts: &[ExpenseReceipt],
//         events: &[ExpenseEvent],
//         groups: &[ExpenseGroup],
//     ) -> RenderedExpense {
//         let transactions = transactions
//             .iter()
//             // .filter(|(t, _)| t.expense_id == info.id)
//             .skip_while(|(t, _, _)| t.expense_id != info.id)
//             .take_while(|(t, _, _)| t.expense_id == info.id)
//             .map(|(t, a, acs)| {
//                 let mut t = t.clone();

//                 if let Some(acs) = acs {
//                     if acs.invert {
//                         t.amount = t.amount.map(|y| -y);
//                         t.fraction = t.fraction.map(|y| -y);
//                     }

//                     t.account_id = if acs.account1 == t.account_id {
//                         acs.account2
//                     } else {
//                         acs.account1
//                     };
//                 }
//                 (t, a.clone(), acs.clone())
//             })
//             .collect::<Vec<_>>();

//         let categories = categories
//             .iter()
//             // .filter(|(c, _)| c.expense_id == info.id)
//             .skip_while(|(t, _)| t.expense_id != info.id)
//             .take_while(|(t, _)| t.expense_id == info.id)
//             .map(|(c, repl)| {
//                 let mut c = c.clone();

//                 if let Some(repl) = repl {
//                     c.category_id = repl.replacement;
//                 }
//                 c
//             })
//             .collect::<Vec<ExpenseCategory>>();

//         let events = events
//             .iter()
//             // .filter(|e| e.expense_id == info.id)
//             .skip_while(|t| t.expense_id != info.id)
//             .take_while(|t| t.expense_id == info.id)
//             .cloned()
//             .collect();

//         let receipts = receipts
//             .iter()
//             // .filter(|r| r.expense_id == info.id)
//             .skip_while(|t| t.expense_id != info.id)
//             .take_while(|t| t.expense_id == info.id)
//             .cloned()
//             .collect();

//         let (total_amount, calculated_amounts) = calculate_total_amount(uid, &info, &transactions);
//         let transactions = transactions
//             .into_iter()
//             .map(|(t, _, _)| t)
//             .collect::<Vec<_>>();

//         let positions = positions
//             .iter()
//             .skip_while(|t| t.expense_id != info.id)
//             .take_while(|t| t.expense_id == info.id)
//             .cloned()
//             .collect();

//         let group = if let Some(g_id) = info.group_id {
//             groups.iter().find(|g| g.id == g_id).cloned()
//         } else {
//             None
//         };

//         RenderedExpense {
//             info,
//             categories,
//             events,
//             transactions,
//             positions,
//             receipts,
//             group,
//             total_amount,
//             calculated_amounts,
//         }
//     }
// }
