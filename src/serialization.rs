use crate::enums::*;
use crate::models::*;
use crate::schema::*;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use itertools::izip;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct NativeFormat {
    users: Vec<User>,
    accounts: Vec<Account>,
    account_synchronizations: Vec<AccountSynchronization>,
    balances: Vec<Balance>,
    categories: Vec<Category>,
    category_replacements: Vec<CategoryReplacement>,
    expenses: Vec<Expense>,
    expense_categories: Vec<ExpenseCategory>,
    expense_transactions: Vec<ExpenseTransaction>,
    expense_receipts: Vec<ExpenseReceipt>,
    delivery_rules: Vec<DeliveryRule>,
}

impl NativeFormat {
    pub fn new(connection: &PgConnection) -> Self {
        let users = users::table
            .load::<User>(connection)
            .expect("Error loading users");
        let accounts = accounts::table
            .load::<Account>(connection)
            .expect("Error loading accounts");
        let account_synchronizations = account_synchronizations::table
            .load::<AccountSynchronization>(connection)
            .expect("Error loading account_synchronizations");
        let balances = balances::table
            .load::<Balance>(connection)
            .expect("Error loading balances");
        let categories = categories::table
            .load::<Category>(connection)
            .expect("Error loading categories");
        let category_replacements = category_replacements::table
            .load::<CategoryReplacement>(connection)
            .expect("Error loading category_replacements");
        let expenses = expenses::table
            .load::<Expense>(connection)
            .expect("Error loading expenses");
        let expense_categories = expense_categories::table
            .load::<ExpenseCategory>(connection)
            .expect("Error loading expense_categories");
        let expense_transactions = expense_transactions::table
            .load::<ExpenseTransaction>(connection)
            .expect("Error loading expense_transactions");
        let expense_receipts = expense_receipts::table
            .load::<ExpenseReceipt>(connection)
            .expect("Error loading expense_receipts");
        let delivery_rules = delivery_rules::table
            .load::<DeliveryRule>(connection)
            .expect("Error loading delivery_rules");

        NativeFormat {
            users,
            accounts,
            account_synchronizations,
            balances,
            categories,
            category_replacements,
            expenses,
            expense_categories,
            expense_transactions,
            expense_receipts,
            delivery_rules,
        }
    }

    pub fn write_to(&self, connection: &PgConnection) {
        let users_count = diesel::insert_into(users::table)
            .values(&self.users)
            .on_conflict_do_nothing()
            .execute(connection)
            .expect("Error writing users into the database");
        info!(
            "imported {} of {} users into the database",
            users_count,
            self.users.len()
        );

        let accounts_count = diesel::insert_into(accounts::table)
            .values(&self.accounts)
            .on_conflict_do_nothing()
            .execute(connection)
            .expect("Error writing accounts into the database");
        info!(
            "imported {} of {} accounts into the database",
            accounts_count,
            self.accounts.len()
        );

        let account_synchronizations_count = diesel::insert_into(account_synchronizations::table)
            .values(&self.account_synchronizations)
            .on_conflict_do_nothing()
            .execute(connection)
            .expect("Error writing account_synchronizations into the database");
        info!(
            "imported {} of {} account_synchronizations into the database",
            account_synchronizations_count,
            self.account_synchronizations.len()
        );

        let balances_count = diesel::insert_into(balances::table)
            .values(&self.balances)
            .on_conflict_do_nothing()
            .execute(connection)
            .expect("Error writing balances into the database");
        info!(
            "imported {} of {} balances into the database",
            balances_count,
            self.balances.len()
        );

        let categories_count = diesel::insert_into(categories::table)
            .values(&self.categories)
            .on_conflict_do_nothing()
            .execute(connection)
            .expect("Error writing categories into the database");
        info!(
            "imported {} of {} categories into the database",
            categories_count,
            self.categories.len()
        );

        let category_replacements_count = diesel::insert_into(category_replacements::table)
            .values(&self.category_replacements)
            .on_conflict_do_nothing()
            .execute(connection)
            .expect("Error writing category_replacements into the database");
        info!(
            "imported {} of {} category_replacements into the database",
            category_replacements_count,
            self.category_replacements.len()
        );

        let expenses_count = diesel::insert_into(expenses::table)
            .values(&self.expenses)
            .on_conflict_do_nothing()
            .execute(connection)
            .expect("Error writing expenses into the database");
        info!(
            "imported {} of {} expenses into the database",
            expenses_count,
            self.expenses.len()
        );

        let expense_categories_count = diesel::insert_into(expense_categories::table)
            .values(&self.expense_categories)
            .on_conflict_do_nothing()
            .execute(connection)
            .expect("Error writing expense_categories into the database");
        info!(
            "imported {} of {} expense_categories into the database",
            expense_categories_count,
            self.expense_categories.len()
        );

        let expense_transactions_count = diesel::insert_into(expense_transactions::table)
            .values(&self.expense_transactions)
            .on_conflict_do_nothing()
            .execute(connection)
            .expect("Error writing expense_transactions into the database");
        info!(
            "imported {} of {} expense_transactions into the database",
            expense_transactions_count,
            self.expense_transactions.len()
        );

        let expense_receipts_count = diesel::insert_into(expense_receipts::table)
            .values(&self.expense_receipts)
            .on_conflict_do_nothing()
            .execute(connection)
            .expect("Error writing expense_receipts into the database");
        info!(
            "imported {} of {} expense_receipts into the database",
            expense_receipts_count,
            self.expense_receipts.len()
        );

        let delivery_rules_count = diesel::insert_into(delivery_rules::table)
            .values(&self.delivery_rules)
            .on_conflict_do_nothing()
            .execute(connection)
            .expect("Error writing delivery_rules into the database");
        info!(
            "imported {} of {} delivery_rules into the database",
            delivery_rules_count,
            self.delivery_rules.len()
        );
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MoneyDBFormat {
    users: Vec<MoneyDBUser>,
    accounts: Vec<MoneyDBAccount>,
    account_syncings: Vec<MoneyDBAccountSyncing>,
    categories: Vec<MoneyDBCategory>,
    category_replacements: Vec<MoneyDBCategoryReplacement>,
    expenses: Vec<MoneyDBExpense>,
    expense_flags: Vec<MoneyDBExpenseFlag>,
    expense_sharings: Vec<MoneyDBExpenseSharing>,
    automation_rules: Vec<MoneyDBAutomationRule>,
    balances: Vec<MoneyDBBalance>,
}

impl MoneyDBFormat {
    pub fn write_to(&self, connection: &PgConnection) {
        let users = diesel::insert_into(users::table)
            .values(self.users.iter().map(|u| u.convert()).collect::<Vec<_>>())
            .load::<User>(connection)
            .expect("Error writing users into the database");
        let user_map = users
            .iter()
            .zip(&self.users)
            .map(|(n, o)| (o.id, n.id))
            .collect::<HashMap<_, _>>();
        info!("imported {} users into the database", users.len());

        let accounts = diesel::insert_into(accounts::table)
            .values(
                self.accounts
                    .iter()
                    .map(|a| a.convert(&user_map))
                    .collect::<Vec<_>>(),
            )
            .load::<Account>(connection)
            .expect("Error writing accounts into the database");
        let account_map = accounts
            .iter()
            .zip(&self.accounts)
            .map(|(n, o)| (o.id, n.id))
            .collect::<HashMap<_, _>>();
        info!("imported {} accounts into the database", accounts.len());

        let account_syncs = diesel::insert_into(account_synchronizations::table)
            .values(
                self.account_syncings
                    .iter()
                    .map(|a| a.convert(&account_map, &accounts))
                    .collect::<Vec<_>>(),
            )
            .load::<AccountSynchronization>(connection)
            .expect("Error writing account synchronizations into the database");
        info!(
            "imported {} account synchronizations into the database",
            account_syncs.len()
        );

        // verify that each account has at most one synchronization
        let mut synced_accounts = account_syncs
            .iter()
            .map(|s| vec![s.account1, s.account2])
            .flatten()
            .collect::<Vec<_>>();
        synced_accounts.sort();
        synced_accounts.dedup();

        if synced_accounts.len() != 2 * account_syncs.len() {
            error!("Some accounts are in multiple synchronization relationships ({} account syncings involving {} accounts). This is unsupported!", account_syncs.len(), synced_accounts.len());
        }

        let categories = diesel::insert_into(categories::table)
            .values(
                self.categories
                    .iter()
                    .map(|a| a.convert(&user_map))
                    .collect::<Vec<_>>(),
            )
            .load::<Category>(connection)
            .expect("Error writing categories into the database");
        let category_map = categories
            .iter()
            .zip(&self.categories)
            .map(|(n, o)| (o.id, n.id))
            .collect::<HashMap<_, _>>();
        info!("imported {} categories into the database", categories.len());

        let num_cat_repl = diesel::insert_into(category_replacements::table)
            .values(
                self.category_replacements
                    .iter()
                    .map(|a| a.convert(&user_map, &category_map))
                    .collect::<Vec<_>>(),
            )
            .execute(connection)
            .expect("Error writing category replacements into the database");
        info!(
            "imported {} category replacements into the database",
            num_cat_repl
        );

        let num_balances = diesel::insert_into(balances::table)
            .values(
                self.balances
                    .iter()
                    .map(|a| a.convert(&account_map))
                    .collect::<Vec<_>>(),
            )
            .execute(connection)
            .expect("Error writing balances into the database");
        info!("imported {} balances into the database", num_balances);

        let expenses = diesel::insert_into(expenses::table)
            .values(
                self.expenses
                    .iter()
                    .map(|a| a.convert(&self.expense_flags))
                    .collect::<Vec<_>>(),
            )
            .load::<Expense>(connection)
            .expect("Error writing expenses into the database");
        let expense_map = expenses
            .iter()
            .zip(&self.expenses)
            .map(|(n, o)| (o.id, n.id))
            .collect::<HashMap<_, _>>();
        info!("imported {} expenses into the database", expenses.len());

        let num_expense_categories = diesel::insert_into(expense_categories::table)
            .values(
                self.expenses
                    .iter()
                    .map(|a| a.convert_category(&expense_map, &category_map))
                    .collect::<Vec<_>>(),
            )
            .execute(connection)
            .expect("Error writing expense categories into the database");
        info!(
            "imported {} expense categories into the database",
            num_expense_categories
        );

        let num_expense_events = diesel::insert_into(expense_events::table)
            .values(
                self.expenses
                    .iter()
                    .map(|a| a.convert_history(&user_map, &expense_map))
                    .collect::<Vec<_>>(),
            )
            .execute(connection)
            .expect("Error writing expense events into the database");
        info!(
            "imported {} expense events into the database",
            num_expense_events
        );

        let converted_transactions = self
            .expenses
            .iter()
            .map(|a| a.convert_transactions(&expense_map, &account_map, &self.expense_sharings))
            .collect::<Vec<_>>();

        let num_expense_transactions = diesel::insert_into(expense_transactions::table)
            .values(converted_transactions.iter().flatten().collect::<Vec<_>>())
            .execute(connection)
            .expect("Error writing expense transactions into the database");
        info!(
            "imported {} expense transactions into the database",
            num_expense_transactions
        );

        // verify that each imported expense has something to do with the user_id of the original expense
        for (old_exp, new_exp, transactions) in
            izip!(&self.expenses, &expenses, &converted_transactions)
        {
            let uid = user_map.get(&old_exp.owner_id).unwrap();
            if !transactions.iter().any(|t| {
                let acc = accounts.iter().find(|a| a.id == t.account_id).unwrap();
                if &acc.user_id == uid {
                    return true;
                }

                if let Some(sync) = account_syncs
                    .iter()
                    .find(|a| a.account1 == t.account_id || a.account2 == t.account_id)
                {
                    let other_id = if sync.account1 == t.account_id {
                        sync.account2
                    } else {
                        sync.account1
                    };
                    let other = accounts.iter().find(|a| a.id == other_id).unwrap();
                    return &other.user_id == uid;
                } else {
                    false
                }
            }) {
                error!("expense {} (new id: {}) will not be accessible by its current owner {} (new id: {}). This is unsupported!", old_exp.id, new_exp.id, old_exp.owner_id, uid);
            }
        }

        warn!("Note that the passwords of all imported users need to be renewed!")
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MoneyDBUser {
    id: i32,
    full_name: String,
    name: String,
}

impl MoneyDBUser {
    fn convert(&self) -> NewUser {
        NewUser {
            name: self.name.to_string(),
            full_name: self.full_name.to_string(),
            hash: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum MoneyDBAvailability {
    Immediately,
    Weeks,
    Months,
    Years,
    Decades,
}

impl MoneyDBAvailability {
    fn convert(&self) -> AccountAvailability {
        match &self {
            MoneyDBAvailability::Immediately => AccountAvailability::Immediately,
            MoneyDBAvailability::Weeks => AccountAvailability::Weeks,
            MoneyDBAvailability::Months => AccountAvailability::Months,
            MoneyDBAvailability::Years => AccountAvailability::Years,
            MoneyDBAvailability::Decades => AccountAvailability::Decades,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum MoneyDBAccountKind {
    Cash,
    Credit,
    Debit,
    Debt,
    Virtual,
    Investment,
    Prepayment,
}

impl MoneyDBAccountKind {
    fn convert(&self) -> AccountKind {
        match &self {
            MoneyDBAccountKind::Cash => AccountKind::Cash,
            MoneyDBAccountKind::Credit => AccountKind::Credit,
            MoneyDBAccountKind::Debit => AccountKind::Debit,
            MoneyDBAccountKind::Debt => AccountKind::Debt,
            MoneyDBAccountKind::Virtual => AccountKind::Virtual,
            MoneyDBAccountKind::Investment => AccountKind::Stocks,
            MoneyDBAccountKind::Prepayment => AccountKind::Other,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MoneyDBAccount {
    id: i32,
    owner_id: i32,
    title: String,
    description: String,
    color: String,
    kind: MoneyDBAccountKind,
    availability: MoneyDBAvailability,
    iban: Option<String>,
    hidden: bool,
}

impl MoneyDBAccount {
    fn convert(&self, user_map: &HashMap<i32, i32>) -> NewAccount {
        NewAccount {
            user_id: *user_map.get(&self.owner_id).unwrap(),
            name: self.title.clone(),
            description: self.description.clone(),
            color: if self.color.is_empty() {
                None
            } else {
                Some(self.color.clone())
            },
            iban: self.iban.clone(),
            kind: self.kind.convert(),
            availability: self.availability.convert(),
            risk: AccountRisk::Medium,
            hidden: self.hidden,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MoneyDBAccountSyncing {
    account1: i32,
    account2: i32,
    sign: bool,
}

impl MoneyDBAccountSyncing {
    fn convert(
        &self,
        account_map: &HashMap<i32, i32>,
        accounts: &[Account],
    ) -> AccountSynchronization {
        let account1 = *account_map.get(&self.account1).unwrap();
        let account2 = *account_map.get(&self.account2).unwrap();

        AccountSynchronization {
            account1,
            account2,
            user1: accounts.iter().find(|a| a.id == account1).unwrap().user_id,
            user2: accounts.iter().find(|a| a.id == account2).unwrap().user_id,
            invert: !self.sign,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MoneyDBCategory {
    id: i32,
    owner_id: i32,
    title: String,
    description: String,
    color: String,
}

impl MoneyDBCategory {
    fn convert(&self, user_map: &HashMap<i32, i32>) -> NewCategory {
        NewCategory {
            user_id: *user_map.get(&self.owner_id).unwrap(),
            name: self.title.clone(),
            description: self.description.clone(),
            color: if self.color.is_empty() {
                None
            } else {
                Some(self.color.clone())
            },
            parent: None,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MoneyDBBalance {
    id: i32,
    account_id: i32,
    date: DateTime<Utc>,
    amount: f64,
}

impl MoneyDBBalance {
    fn convert(&self, account_map: &HashMap<i32, i32>) -> NewBalance {
        NewBalance {
            account_id: *account_map.get(&self.account_id).unwrap(),
            date: self.date,
            amount: (self.amount * 100.0) as i64,
            comment: String::new(),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MoneyDBCategoryReplacement {
    owner_id: i32,
    original: i32,
    replacement: i32,
}

impl MoneyDBCategoryReplacement {
    fn convert(
        &self,
        user_map: &HashMap<i32, i32>,
        category_map: &HashMap<i32, i32>,
    ) -> CategoryReplacement {
        CategoryReplacement {
            user_id: *user_map.get(&self.owner_id).unwrap(),
            original: *category_map.get(&self.original).unwrap(),
            replacement: *category_map.get(&self.replacement).unwrap(),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MoneyDBExpense {
    id: i32,
    owner_id: i32,
    amount: f64,
    value_date: DateTime<Utc>,
    booking_date: Option<DateTime<Utc>>,
    account_id: i32,
    category_id: i32,
    title: String,
    description: String,
    store: String,
    transaction: String,
    comments: String,
    creation_date: DateTime<Utc>,
    last_modified: DateTime<Utc>,
    last_modified_through: String,
    last_modified_by: i32,
}

impl MoneyDBExpense {
    fn convert(&self, flags: &[MoneyDBExpenseFlag]) -> NewExpense {
        NewExpense {
            title: self.title.clone(),
            description: self.description.clone(),
            comments: self.comments.clone(),
            store: self.store.clone(),
            booking_start: self.booking_date.unwrap_or(self.value_date),
            booking_end: self.booking_date.unwrap_or(self.value_date),
            is_deleted: false,
            is_template: flags
                .iter()
                .any(|f| f.flagging == MoneyDBExpenseFlagging::Template && f.expense_id == self.id),
            is_preliminary: flags.iter().any(|f| {
                f.flagging == MoneyDBExpenseFlagging::Preliminary && f.expense_id == self.id
            }),
            is_tax_relevant: flags.iter().any(|f| {
                f.flagging == MoneyDBExpenseFlagging::TaxRelevant && f.expense_id == self.id
            }),
            is_unchecked: flags.iter().any(|f| {
                f.flagging == MoneyDBExpenseFlagging::NeedsAttention && f.expense_id == self.id
            }),
        }
    }

    fn convert_category(
        &self,
        expense_map: &HashMap<i32, i32>,
        category_map: &HashMap<i32, i32>,
    ) -> ExpenseCategory {
        ExpenseCategory {
            weight: 1.0,
            category_id: *category_map.get(&self.category_id).unwrap(),
            expense_id: *expense_map.get(&self.id).unwrap(),
        }
    }

    fn convert_transactions(
        &self,
        expense_map: &HashMap<i32, i32>,
        account_map: &HashMap<i32, i32>,
        sharings: &[MoneyDBExpenseSharing],
    ) -> Vec<NewExpenseTransaction> {
        let mut res = Vec::new();

        res.push(NewExpenseTransaction {
            expense_id: *expense_map.get(&self.id).unwrap(),
            account_id: *account_map.get(&self.account_id).unwrap(),
            statement: self.transaction.clone(),
            comments: String::new(),
            date: self.value_date,
            amount: Some((self.amount * -100.0) as i64),
            fraction: None,
        });

        let num_equal = sharings
            .iter()
            .filter(|s| {
                s.sharing_type == MoneyDBExpenseSharingType::Equal && s.expense_id == self.id
            })
            .collect::<Vec<_>>()
            .len();

        // in the new version percentage is only allowed if exactly one transaction has an amount
        let any_fixed_amount = sharings.iter().any(|s| {
            s.sharing_type == MoneyDBExpenseSharingType::FixedAmount && s.expense_id == self.id
        });

        for s in sharings.into_iter().filter(|s| s.expense_id == self.id) {
            let mut t = NewExpenseTransaction {
                expense_id: *expense_map.get(&self.id).unwrap(),
                account_id: *account_map.get(&s.account_id).unwrap(),
                statement: String::new(),
                comments: String::new(),
                date: self.value_date,
                amount: None,
                fraction: None,
            };

            if s.sharing_type == MoneyDBExpenseSharingType::FixedAmount {
                t.amount = Some((s.param * 100.0) as i64)
            } else if s.sharing_type == MoneyDBExpenseSharingType::Equal {
                if !any_fixed_amount {
                    t.fraction = Some(-1.0 / (num_equal + 1) as f64);
                } else {
                    // warn!("expense {}: converting equal to fixed amount", &self.id);
                    t.amount = Some((self.amount / (num_equal + 1) as f64 * -100.0) as i64);
                }
            } else if s.sharing_type == MoneyDBExpenseSharingType::FixedFraction {
                if !any_fixed_amount {
                    t.fraction = Some(-1.0 * s.param);
                } else {
                    // warn!("expense {}: converting fixed fraction to fixed amount", &self.id);
                    t.amount = Some((self.amount * s.param * -100.0) as i64);
                }
            }

            res.push(t);
        }

        res
    }

    fn convert_history(
        &self,
        user_map: &HashMap<i32, i32>,
        expense_map: &HashMap<i32, i32>,
    ) -> NewExpenseEvent {
        NewExpenseEvent {
            expense_id: *expense_map.get(&self.id).unwrap(),
            user_id: *user_map.get(&self.last_modified_by).unwrap(),
            date: self.last_modified,
            tool: self.last_modified_through.clone(),
            automatic: false,
            event_type: ExpenseEventType::Modify,
            event_target: ExpenseEventTarget::Expense,
            payload: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub enum MoneyDBExpenseSharingType {
    Equal,
    FixedAmount,
    FixedFraction,
}
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub enum MoneyDBExpenseFlagging {
    Template,
    NeedsAttention,
    TaxRelevant,
    Compensated,
    Preliminary,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MoneyDBExpenseSharing {
    expense_id: i32,
    account_id: i32,
    #[serde(rename = "type")]
    sharing_type: MoneyDBExpenseSharingType,
    param: f64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MoneyDBExpenseFlag {
    expense_id: i32,
    flagging: MoneyDBExpenseFlagging,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MoneyDBAutomationRule {
    owner_id: i32,
    title: String,
    template_id: i32,
    priority: i32,
    regex_time: Option<String>,
    filter_account: Option<i32>,
    regex_transaction: Option<String>,
    filter_amount: Option<f64>,
    last_delivery: Option<DateTime<Utc>>,
}
