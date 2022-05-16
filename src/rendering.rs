use crate::models::{
    rendering::{RenderedBalance, RenderedExpense, RenderedExpenseTransaction},
    Account, Balance, Category, Expense, ExpenseTransaction,
};
use itertools::Itertools;
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
impl ExpenseTransaction {
    pub fn from_rendered(transaction: RenderedExpenseTransaction) -> Self {
        Self {
            account_id: transaction.account_id,
            date: transaction.date,
            amount: transaction.amount,
            fraction: transaction.fraction,
            comments: transaction.comments,
            statement: transaction.statement,
        }
    }
}

impl Expense {
    pub fn from_rendered(expense: RenderedExpense) -> Self {
        Self {
            id: ObjectId::new().into(),
            title: expense.title,
            store: expense.store,
            description: expense.description,
            comments: expense.comments,
            booking_start: expense.booking_start,
            booking_end: expense.booking_end,
            recurring_expense_id: expense.recurring_expense_id,
            is_template: expense.is_template,
            is_tax_relevant: expense.is_tax_relevant,
            is_unchecked: expense.is_unchecked,
            is_incomplete: expense.is_incomplete,
            receipts: expense.receipts,
            categories: expense.categories,
            transactions: expense
                .transactions
                .into_iter()
                .map(ExpenseTransaction::from_rendered)
                .collect(),
            positions: expense.positions,
        }
    }

    pub fn render_for_user(
        &self,
        user_id: ObjectId,
        accounts: &[Account],
        categories: &[Category],
    ) -> Result<RenderedExpense, Box<dyn Error>> {
        todo!() // TODO:

        // first: convert all transactions that have a fixed amount

        // second: if necessary, i.e. there is at least one fractional transactions, then there should be exactly one rendered fixed amount transaction.
        // use that amount to convert transactions with a fixed fraction? or should the amount used before rendering??
    }

    pub fn render(
        &self,
        accounts: &[Account],
        categories: &[Category],
    ) -> Result<Vec<RenderedExpense>, Box<dyn Error>> {
        let main_accounts: Vec<_> = accounts
            .iter()
            .filter(|a| self.transactions.iter().any(|t| a.id == t.account_id))
            .collect();

        let synchronized_account_ids: Vec<_> = main_accounts
            .iter()
            .flat_map(|a| a.synchronization.as_ref().map(|ass| ass.account_id.clone()))
            .collect();
        let synchronized_accounts: Vec<_> = accounts
            .iter()
            .filter(|a| synchronized_account_ids.iter().any(|t| &a.id == t))
            .collect();

        let relevant_accounts: Vec<_> = synchronized_accounts
            .iter()
            .chain(main_accounts.iter())
            .cloned()
            .cloned()
            .collect();
        relevant_accounts
            .iter()
            .map(|a| *a.user_id.clone())
            .unique()
            .map(|uid| self.render_for_user(uid, &relevant_accounts, &categories))
            .collect::<Result<Vec<_>, _>>()

        // let mut res = vec![RenderedBalance {
        //     id: ObjectId::new().into(),
        //     upstream_id: self.id.clone(),
        //     user_id: main_account.user_id.clone(),
        //     date: self.date,
        //     account_id: main_account.id.clone(),
        //     amount: self.amount,
        //     comments: self.comments.clone(),
        // }];

        // if let Some(sync) = &main_account.synchronization {
        //     let other_account = accounts
        //         .iter()
        //         .find(|a| a.id == sync.account_id)
        //         .ok_or_else(|| {
        //             format!("Cannot find synchronized account of {:?}", &main_account)
        //         })?;

        //     res.push(RenderedBalance {
        //         id: ObjectId::new().into(),
        //         upstream_id: self.id.clone(),
        //         user_id: other_account.user_id.clone(),
        //         date: self.date,
        //         account_id: other_account.id.clone(),
        //         amount: self.amount * if sync.invert { -1 } else { 1 },
        //         comments: self.comments.clone(),
        //     });
        // }

        // Ok(res)
    }

    pub fn apply_changes(&mut self, changes: &RenderedExpense) {
        self.booking_start = changes.booking_start;
        self.booking_end = changes.booking_end;
        self.title = changes.title.clone();
        self.store = changes.store.clone();
        self.description = changes.description.clone();
        self.comments = changes.comments.clone();
        self.recurring_expense_id = changes.recurring_expense_id.clone();
        self.is_template = changes.is_template;
        self.is_tax_relevant = changes.is_tax_relevant;
        self.is_unchecked = changes.is_unchecked;
        self.is_incomplete = changes.is_incomplete;
        self.receipts = changes.receipts.clone();
        self.categories = changes.categories.clone();
        self.transactions = changes
            .transactions
            .iter()
            .cloned()
            .map(ExpenseTransaction::from_rendered)
            .collect();
        self.positions = changes.positions.clone();
    }

    pub async fn remove_rendered(&self, db: &Database) -> Result<DeleteResult, Box<dyn Error>> {
        let rendered_expenses_collection = db.collection::<RenderedExpense>("expenses_rendered");
        let r = rendered_expenses_collection
            .delete_many(doc!("upstream_id": *self.id.clone()), None)
            .await?;

        info!(
            "deleted {} rendered expenses of expense {:?}",
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

//     // if there is an account syncing, then the synced account has to belong to the user anyway, no need to check user ids in that case
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
