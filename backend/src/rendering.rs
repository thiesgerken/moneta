use crate::models::{
    Account, Balance, Expense, ExpenseCategorizationInput, ExpensePositionInput,
    ExpenseReceiptInput, ExpenseTransaction, ExpenseTransactionInput,
    rendering::{
        RenderedBalance, RenderedBalanceInput, RenderedExpense, RenderedExpenseInput,
        RenderedExpenseTransaction,
    },
};
use chrono::Utc;
use futures::TryStreamExt;
use itertools::Itertools;
use mongodb::{
    Database,
    bson::{doc, oid::ObjectId},
    results::DeleteResult,
};
use std::error::Error;

pub async fn rerender_multiple(
    connection: mongodb::Database,
    account_ids: &[ObjectId],
) -> Result<(), Box<dyn Error>> {
    let b_count = rerender_multiple_balances(connection.clone(), account_ids).await?;
    info!(
        "Rerendered {} balances belonging to {} accounts",
        b_count,
        account_ids.len()
    );

    let e_count = rerender_multiple_expenses(connection.clone(), account_ids).await?;
    info!(
        "Rerendered {} expenses belonging to {} accounts",
        e_count,
        account_ids.len()
    );

    Ok(())
}

/// account_ids must contain synchronized accounts as well
pub async fn rerender_multiple_balances(
    connection: mongodb::Database,
    account_ids: &[ObjectId],
) -> Result<usize, Box<dyn Error>> {
    let account_collection = connection.collection::<Account>("accounts");

    let accounts = account_collection
        .find(doc!("_id": {"$in": &account_ids}))
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    let rendered_balances_collection =
        connection.collection::<RenderedBalance>("balances_rendered");
    rendered_balances_collection
        .delete_many(doc!("accountId": {"$in": &account_ids}))
        .await?;

    let balance_collection = connection.collection::<Balance>("balances");
    let mut cursor = balance_collection
        .find(doc!("accountId": {"$in": &account_ids}))
        .await?;

    let mut rendered_balances = Vec::new();
    while let Some(b) = cursor.try_next().await? {
        rendered_balances.extend(b.render(&accounts)?);
    }

    let r = rendered_balances_collection
        .insert_many(&rendered_balances)
        .await?;

    Ok(r.inserted_ids.len())
}

/// account_ids must contain synchronized accounts as well
pub async fn rerender_multiple_expenses(
    connection: mongodb::Database,
    account_ids: &[ObjectId],
) -> Result<usize, Box<dyn Error>> {
    // need to get all accounts, because the transactions could be linked to other accounts
    let account_collection = connection.collection::<Account>("accounts");
    let accounts = account_collection
        .find(doc!()) // can't be that many, right?
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    let rendered_expenses_collection =
        connection.collection::<RenderedExpense>("expenses_rendered");
    rendered_expenses_collection
        .delete_many(doc!("transactions.accountId": {"$in": &account_ids}))
        .await?;

    let expense_collection = connection.collection::<Expense>("expenses");
    let mut cursor = expense_collection
        .find(doc!("transactions.accountId": {"$in": &account_ids}))
        .await?;

    let mut rendered_expenses = Vec::new();
    while let Some(e) = cursor.try_next().await? {
        rendered_expenses.extend(e.render(&accounts)?);
    }

    let r = rendered_expenses_collection
        .insert_many(&rendered_expenses)
        .await?;

    Ok(r.inserted_ids.len())
}

impl Balance {
    pub fn from_rendered(balance: RenderedBalanceInput) -> Self {
        let now = Utc::now();

        Self {
            id: ObjectId::new().into(),
            date: balance.date,
            account_id: balance.account_id,
            amount: balance.amount,
            comments: balance.comments,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn render(&self, accounts: &[Account]) -> Result<Vec<RenderedBalance>, Box<dyn Error>> {
        let main_account = accounts
            .iter()
            .find(|a| a.id == self.account_id)
            .ok_or_else(|| format!("Cannot find main account of {:?}", &self))?;

        let mut res = vec![RenderedBalance {
            id: ObjectId::new().into(),
            upstream_id: self.id,
            user_id: main_account.user_id,
            date: self.date,
            account_id: main_account.id,
            amount: self.amount,
            comments: self.comments.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
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
                upstream_id: self.id,
                user_id: other_account.user_id,
                date: self.date,
                account_id: other_account.id,
                amount: self.amount * if sync.invert { -1 } else { 1 },
                comments: self.comments.clone(),
                created_at: self.created_at,
                updated_at: self.updated_at,
            });
        }

        Ok(res)
    }

    pub async fn remove_rendered(&self, db: &Database) -> Result<DeleteResult, Box<dyn Error>> {
        let rendered_balances_collection = db.collection::<RenderedBalance>("balances_rendered");
        let r = rendered_balances_collection
            .delete_many(doc!("upstreamId": *self.id))
            .await?;

        info!(
            "deleted {} rendered balances of balance {:?}",
            r.deleted_count, &self.id
        );
        Ok(r)
    }
}
impl ExpenseTransaction {
    pub fn from_rendered(transaction: ExpenseTransactionInput) -> Self {
        Self {
            account_id: transaction.account_id,
            date: transaction.date,
            amount: transaction.amount,
            fraction: transaction.fraction,
            comments: transaction.comments,
            statement: transaction.statement,
        }
    }

    pub fn render_for_user(
        &self,
        user_id: ObjectId,
        accounts: &[Account],
        reference_amount: Option<i32>,
    ) -> Result<RenderedExpenseTransaction, Box<dyn Error>> {
        let mut res = RenderedExpenseTransaction {
            date: self.date,
            account_id: self.account_id,
            comments: self.comments.clone(),
            amount: self.amount,
            fraction: self.fraction,
            statement: self.statement.clone(),
            calculated_amount: 0.0, // will be filled in the sequel
            effective_amount: 0.0,  // will be filled in the sequel
        };

        res.calculated_amount = if let Some(amount) = self.amount {
            amount as f64
        } else {
            let total_amount = reference_amount.ok_or("tried to convert a fractional transaction, but no reference amount available?! expense seems to be invalid")?;

            total_amount as f64
                * self
                    .fraction
                    .ok_or("tried to convert transaction without amount or fraction set")?
        };

        let main_account = accounts
            .iter()
            .find(|a| a.id == self.account_id)
            .ok_or_else(|| format!("cannot find main account of {:?}", self))?;

        res.effective_amount = if *main_account.user_id == user_id {
            main_account.ownership_fraction * res.calculated_amount
        } else {
            0.0
        };

        if let Some(sync) = &main_account.synchronization {
            let other_account = accounts
                .iter()
                .find(|a| a.id == sync.account_id)
                .ok_or_else(|| {
                    format!("Cannot find synchronized account of {:?}", &main_account)
                })?;

            if *other_account.user_id == user_id {
                res.account_id = other_account.id;

                if sync.invert {
                    res.calculated_amount *= -1.;
                    res.amount = res.amount.map(|x| -x);
                    res.fraction = res.fraction.map(|x| -x);
                }

                res.effective_amount = other_account.ownership_fraction * res.calculated_amount;
            }
        }

        Ok(res)
    }
}

impl Expense {
    pub fn from_rendered(expense: RenderedExpenseInput) -> Self {
        let now = Utc::now();

        Self {
            id: ObjectId::new().into(),
            title: expense.title,
            store: expense.store,
            description: expense.description,
            comments: expense.comments,
            booking_start: expense.booking_start,
            booking_end: expense.booking_end,
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

    pub fn render_for_user(
        &self,
        user_id: ObjectId,
        accounts: &[Account],
    ) -> Result<RenderedExpense, Box<dyn Error>> {
        let rendered_ts: Vec<_> = self
            .transactions
            .iter()
            .map(|t| t.render_for_user(user_id, accounts, self.reference_amount))
            .collect::<Result<_, _>>()?;
        let effective_amount = rendered_ts.iter().map(|t| t.effective_amount).sum();

        Ok(RenderedExpense {
            id: ObjectId::new().into(),
            upstream_id: self.id,
            user_id: user_id.into(),
            title: self.title.clone(),
            store: self.store.clone(),
            description: self.description.clone(),
            reference_amount: self.reference_amount,
            comments: self.comments.clone(),
            booking_start: self.booking_start,
            booking_end: self.booking_end,
            is_template: self.is_template,
            is_tax_relevant: self.is_tax_relevant,
            is_unchecked: self.is_unchecked,
            is_incomplete: self.is_incomplete,
            is_extraordinary: self.is_extraordinary,
            receipts: self.receipts.clone(),
            categories: self.categories.clone(),
            transactions: rendered_ts,
            positions: self.positions.clone(),
            effective_amount,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }

    pub fn render(&self, accounts: &[Account]) -> Result<Vec<RenderedExpense>, Box<dyn Error>> {
        let main_accounts: Vec<_> = accounts
            .iter()
            .filter(|a| self.transactions.iter().any(|t| a.id == t.account_id))
            .collect();

        let synchronized_account_ids: Vec<_> = main_accounts
            .iter()
            .flat_map(|a| a.synchronization.as_ref().map(|ass| ass.account_id))
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
            .map(|a| *a.user_id)
            .unique()
            .map(|uid| self.render_for_user(uid, &relevant_accounts))
            .collect::<Result<Vec<_>, _>>()
    }

    pub async fn remove_rendered(&self, db: &Database) -> Result<DeleteResult, Box<dyn Error>> {
        let rendered_expenses_collection = db.collection::<RenderedExpense>("expenses_rendered");
        let r = rendered_expenses_collection
            .delete_many(doc!("upstreamId": *self.id))
            .await?;

        info!(
            "deleted {} rendered expenses of expense {:?}",
            r.deleted_count, &self.id
        );
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        AccountAvailability, AccountKind, AccountRisk, AccountSynchronization, ExpenseTransaction,
    };
    use chrono::TimeZone;

    fn date() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2025, 6, 1, 12, 0, 0).unwrap()
    }

    fn account(user_id: ObjectId, ownership_fraction: f64) -> Account {
        Account {
            id: ObjectId::new().into(),
            user_id: user_id.into(),
            name: "test".into(),
            description: String::new(),
            color: None,
            iban: None,
            kind: AccountKind::Debit,
            availability: AccountAvailability::Immediately,
            risk: AccountRisk::None,
            opened: date(),
            closed: None,
            ownership_fraction,
            synchronization: None,
        }
    }

    /// links `a` and `b` so each renders into the other's owner's view
    fn synchronize(a: &mut Account, b: &mut Account, invert: bool) {
        a.synchronization = Some(AccountSynchronization {
            account_id: b.id,
            invert,
        });
        b.synchronization = Some(AccountSynchronization {
            account_id: a.id,
            invert,
        });
    }

    fn transaction(
        account: &Account,
        amount: Option<i32>,
        fraction: Option<f64>,
    ) -> ExpenseTransaction {
        ExpenseTransaction {
            account_id: account.id,
            date: date(),
            amount,
            fraction,
            comments: String::new(),
            statement: String::new(),
        }
    }

    fn expense(transactions: Vec<ExpenseTransaction>, reference_amount: Option<i32>) -> Expense {
        Expense {
            id: ObjectId::new().into(),
            title: "test".into(),
            store: String::new(),
            description: String::new(),
            comments: String::new(),
            booking_start: date(),
            booking_end: date(),
            reference_amount,
            is_template: false,
            is_tax_relevant: false,
            is_unchecked: false,
            is_incomplete: false,
            is_extraordinary: false,
            receipts: vec![],
            categories: vec![],
            transactions,
            positions: vec![],
            created_at: date(),
            updated_at: date(),
        }
    }

    fn balance(account: &Account, amount: i32) -> Balance {
        Balance {
            id: ObjectId::new().into(),
            date: date(),
            account_id: account.id,
            amount,
            comments: String::new(),
            created_at: date(),
            updated_at: date(),
        }
    }

    #[test]
    fn renders_a_wholly_owned_expense_at_face_value() {
        let uid = ObjectId::new();
        let acc = account(uid, 1.0);
        let e = expense(vec![transaction(&acc, Some(-1250), None)], None);

        let rendered = e.render(&[acc]).unwrap();

        assert_eq!(rendered.len(), 1);
        assert_eq!(rendered[0].effective_amount, -1250.0);
        assert_eq!(rendered[0].transactions[0].calculated_amount, -1250.0);
    }

    /// a jointly owned account only contributes its owner's share to the totals
    #[test]
    fn applies_the_ownership_fraction_to_the_effective_amount() {
        let uid = ObjectId::new();
        let acc = account(uid, 0.5);
        let e = expense(vec![transaction(&acc, Some(-1000), None)], None);

        let rendered = e.render(&[acc]).unwrap();

        // the transaction itself is unchanged, only the user's share of it shrinks
        assert_eq!(rendered[0].transactions[0].calculated_amount, -1000.0);
        assert_eq!(rendered[0].effective_amount, -500.0);
    }

    #[test]
    fn resolves_fractional_transactions_against_the_reference_amount() {
        let uid = ObjectId::new();
        let acc = account(uid, 1.0);
        let e = expense(vec![transaction(&acc, None, Some(0.25))], Some(-2000));

        let rendered = e.render(&[acc]).unwrap();

        assert_eq!(rendered[0].transactions[0].calculated_amount, -500.0);
    }

    #[test]
    fn rejects_a_fractional_transaction_without_a_reference_amount() {
        let uid = ObjectId::new();
        let acc = account(uid, 1.0);
        let e = expense(vec![transaction(&acc, None, Some(0.25))], None);

        assert!(e.render(&[acc]).is_err());
    }

    #[test]
    fn sums_effective_amounts_across_transactions() {
        let uid = ObjectId::new();
        let acc = account(uid, 1.0);
        let e = expense(
            vec![
                transaction(&acc, Some(-1000), None),
                transaction(&acc, Some(-500), None),
            ],
            None,
        );

        let rendered = e.render(&[acc]).unwrap();

        assert_eq!(rendered[0].effective_amount, -1500.0);
    }

    /// an expense touching two users' accounts is stored once per user
    #[test]
    fn renders_one_document_per_involved_user() {
        let (mine, theirs) = (ObjectId::new(), ObjectId::new());
        let my_account = account(mine, 1.0);
        let their_account = account(theirs, 1.0);

        let e = expense(
            vec![
                transaction(&my_account, Some(-1000), None),
                transaction(&their_account, Some(-400), None),
            ],
            None,
        );

        let rendered = e.render(&[my_account, their_account]).unwrap();
        assert_eq!(rendered.len(), 2);

        // each user sees only what their own account contributed
        let for_me = rendered.iter().find(|r| *r.user_id == mine).unwrap();
        let for_them = rendered.iter().find(|r| *r.user_id == theirs).unwrap();
        assert_eq!(for_me.effective_amount, -1000.0);
        assert_eq!(for_them.effective_amount, -400.0);
    }

    /// an inverted sync models a debt: what one side pays, the other side owes
    #[test]
    fn inverted_synchronization_flips_the_sign_for_the_other_user() {
        let (mine, theirs) = (ObjectId::new(), ObjectId::new());
        let mut my_account = account(mine, 1.0);
        let mut their_account = account(theirs, 1.0);
        synchronize(&mut my_account, &mut their_account, true);

        let e = expense(vec![transaction(&my_account, Some(-1000), None)], None);
        let rendered = e
            .render(&[my_account.clone(), their_account.clone()])
            .unwrap();

        assert_eq!(rendered.len(), 2);
        let for_me = rendered.iter().find(|r| *r.user_id == mine).unwrap();
        let for_them = rendered.iter().find(|r| *r.user_id == theirs).unwrap();

        assert_eq!(for_me.effective_amount, -1000.0);
        assert_eq!(for_them.effective_amount, 1000.0);
        // the other user sees it booked against their own account, not mine
        assert_eq!(for_them.transactions[0].account_id, their_account.id);
        assert_eq!(for_them.transactions[0].amount, Some(1000));
    }

    #[test]
    fn plain_synchronization_keeps_the_sign_for_the_other_user() {
        let (mine, theirs) = (ObjectId::new(), ObjectId::new());
        let mut my_account = account(mine, 1.0);
        let mut their_account = account(theirs, 1.0);
        synchronize(&mut my_account, &mut their_account, false);

        let e = expense(vec![transaction(&my_account, Some(-1000), None)], None);
        let rendered = e.render(&[my_account, their_account]).unwrap();

        let for_them = rendered.iter().find(|r| *r.user_id == theirs).unwrap();
        assert_eq!(for_them.effective_amount, -1000.0);
    }

    #[test]
    fn renders_a_balance_for_its_owner() {
        let uid = ObjectId::new();
        let acc = account(uid, 1.0);
        let b = balance(&acc, 5000);

        let rendered = b.render(std::slice::from_ref(&acc)).unwrap();

        assert_eq!(rendered.len(), 1);
        assert_eq!(rendered[0].amount, 5000);
        assert_eq!(*rendered[0].user_id, uid);
        assert_eq!(rendered[0].upstream_id, b.id);
    }

    #[test]
    fn inverted_synchronization_flips_the_balance_for_the_other_user() {
        let (mine, theirs) = (ObjectId::new(), ObjectId::new());
        let mut my_account = account(mine, 1.0);
        let mut their_account = account(theirs, 1.0);
        synchronize(&mut my_account, &mut their_account, true);

        let b = balance(&my_account, 5000);
        let rendered = b.render(&[my_account, their_account]).unwrap();

        assert_eq!(rendered.len(), 2);
        assert_eq!(
            rendered.iter().find(|r| *r.user_id == mine).unwrap().amount,
            5000
        );
        assert_eq!(
            rendered
                .iter()
                .find(|r| *r.user_id == theirs)
                .unwrap()
                .amount,
            -5000
        );
    }

    #[test]
    fn fails_when_an_account_referenced_by_a_transaction_is_missing() {
        let uid = ObjectId::new();
        let acc = account(uid, 1.0);
        let e = expense(vec![transaction(&acc, Some(-100), None)], None);

        // rendering without the account it points at must not silently produce nothing
        assert!(e.render_for_user(uid, &[]).is_err());
    }
}
