//! The whole database's user data as one document, for `moneta export` and `moneta import`.
//!
//! What it leaves out is credentials: password hashes, API keys and renditier tokens. Sessions
//! are signed cookies and never were in the database. The rendered views derive from the
//! tables written here, so there is nothing else to rebuild after an import.
//!
//! Ids are kept as they are. The import only ever writes into a database without user data, so
//! they cannot collide, and keeping them is what keeps the changelog meaningful: its snapshots
//! name expenses, accounts and categories by id inside jsonb, and `target_id` is deliberately
//! not a foreign key, so fresh ids would quietly detach the history from what it describes.

use std::collections::{HashMap, HashSet};
use std::error::Error;

use chrono::{DateTime, Utc};
use diesel::dsl::exists;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{
    Account, AutomationRule, Balance, Category, Expense, ExpenseCategorizationRow, ExpenseRow,
    ExpenseTransactionRow, PeriodicRule, SeenDelivery,
};
use crate::schema::{
    accounts, automation_rules, balances, categories, expense_categorizations, expense_changes,
    expense_transactions, expenses, periodic_rules, seen_deliveries, users,
};

/// Stays below postgres' limit of 65535 bind parameters per statement for every table here.
const ROWS_PER_INSERT: usize = 1000;

/// What an imported user's password hash is set to. It is no PHC string, so `verify_password`
/// rejects every password against it, and the user cannot sign in until `moneta user --update`
/// gives them one.
const NO_PASSWORD: &str = "";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Backup {
    pub users: Vec<BackupUser>,
    pub categories: Vec<Category>,
    pub accounts: Vec<Account>,
    pub expenses: Vec<Expense>,
    pub balances: Vec<Balance>,
    pub periodic_rules: Vec<PeriodicRule>,
    pub automation_rules: Vec<AutomationRule>,
    pub seen_deliveries: Vec<SeenDelivery>,
    pub expense_changes: Vec<BackupExpenseChange>,
}

/// A user without their credentials.
#[derive(Debug, Serialize, Deserialize, Queryable, Selectable)]
#[diesel(table_name = users, check_for_backend(diesel::pg::Pg))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupUser {
    pub id: Uuid,
    pub name: String,
    pub full_name: String,
    pub seen_at: Option<DateTime<Utc>>,
    pub seen_before: Option<DateTime<Utc>>,
}

/// A changelog entry with its snapshots as the documents they are stored as. Parsing them into
/// today's [`Expense`] would rewrite every snapshot written under an older shape of it, which is
/// exactly what keeping them as documents is meant to prevent.
#[derive(Debug, Serialize, Deserialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = expense_changes, check_for_backend(diesel::pg::Pg))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupExpenseChange {
    pub id: Uuid,
    pub target_id: Uuid,
    pub user_id: Option<Uuid>,
    pub date: DateTime<Utc>,
    pub tool: Option<String>,
    pub automatic: bool,
    pub confidence: f64,
    pub old_state: Option<serde_json::Value>,
    pub new_state: Option<serde_json::Value>,
}

impl Backup {
    /// Reads everything, each table in id order so that two exports of the same data are
    /// byte for byte the same.
    pub async fn load(conn: &mut AsyncPgConnection) -> QueryResult<Self> {
        Ok(Self {
            users: users::table
                .order_by(users::id)
                .select(BackupUser::as_select())
                .load(conn)
                .await?,
            categories: categories::table
                .order_by(categories::id)
                .select(Category::as_select())
                .load(conn)
                .await?,
            accounts: accounts::table
                .order_by(accounts::id)
                .select(Account::as_select())
                .load(conn)
                .await?,
            expenses: load_expenses(conn).await?,
            balances: balances::table
                .order_by(balances::id)
                .select(Balance::as_select())
                .load(conn)
                .await?,
            periodic_rules: periodic_rules::table
                .order_by(periodic_rules::id)
                .select(PeriodicRule::as_select())
                .load(conn)
                .await?,
            automation_rules: automation_rules::table
                .order_by(automation_rules::id)
                .select(AutomationRule::as_select())
                .load(conn)
                .await?,
            seen_deliveries: seen_deliveries::table
                .order_by(seen_deliveries::id)
                .select(SeenDelivery::as_select())
                .load(conn)
                .await?,
            expense_changes: expense_changes::table
                .order_by(expense_changes::id)
                .select(BackupExpenseChange::as_select())
                .load(conn)
                .await?,
        })
    }

    /// Writes the backup into a database that holds no user data yet, in one transaction.
    ///
    /// Only into an empty one: an expense books across several users' accounts, so there is no
    /// way to replace one user's data without breaking the bookings shared with the others.
    /// Anything wrong with the backup is reported before the first write, and anything the
    /// database rejects after that rolls the whole import back.
    pub async fn restore(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.check()?;

        conn.transaction::<_, Box<dyn Error + Send + Sync>, _>(async |conn| {
            if holds_user_data(conn).await? {
                return Err("the database already holds user data; \
                            import only restores into an empty one"
                    .into());
            }
            self.insert(conn).await?;
            Ok(())
        })
        .await
    }

    /// Rejects what would make the import ambiguous or that the database cannot say clearly:
    /// duplicate ids, references to rows the backup does not contain, and expenses that break
    /// the rules the API enforces on every write. Per-row value rules are left to the CHECK
    /// constraints, which name themselves when they fail and roll the import back.
    fn check(&self) -> Result<(), String> {
        let user_ids = unique("user", self.users.iter().map(|u| u.id))?;
        let mut names = HashSet::new();
        if let Some(user) = self.users.iter().find(|u| !names.insert(&u.name)) {
            return Err(format!("user name '{}' appears twice", user.name));
        }

        let category_ids = unique("category", self.categories.iter().map(|c| c.id))?;
        for c in &self.categories {
            refers("category", c.id, "parent", c.parent_id, &category_ids)?;
        }

        let account_ids = unique("account", self.accounts.iter().map(|a| a.id))?;
        for a in &self.accounts {
            refers("account", a.id, "user", Some(a.user_id), &user_ids)?;
            refers(
                "account",
                a.id,
                "synced account",
                a.sync_account_id,
                &account_ids,
            )?;
        }

        let expense_ids = unique("expense", self.expenses.iter().map(|e| e.id))?;
        for e in &self.expenses {
            e.validate()
                .map_err(|error| format!("expense {}: {error}", e.id))?;
            for t in &e.transactions {
                refers("expense", e.id, "account", Some(t.account_id), &account_ids)?;
            }
            for c in &e.categories {
                refers(
                    "expense",
                    e.id,
                    "category",
                    Some(c.category_id),
                    &category_ids,
                )?;
            }
        }

        unique("balance", self.balances.iter().map(|b| b.id))?;
        for b in &self.balances {
            refers("balance", b.id, "account", Some(b.account_id), &account_ids)?;
        }

        unique("periodic rule", self.periodic_rules.iter().map(|r| r.id))?;
        for r in &self.periodic_rules {
            refers("periodic rule", r.id, "user", Some(r.user_id), &user_ids)?;
            refers(
                "periodic rule",
                r.id,
                "template",
                Some(r.template_id),
                &expense_ids,
            )?;
        }

        unique(
            "automation rule",
            self.automation_rules.iter().map(|r| r.id),
        )?;
        for r in &self.automation_rules {
            refers("automation rule", r.id, "user", Some(r.user_id), &user_ids)?;
            refers(
                "automation rule",
                r.id,
                "template",
                Some(r.template_id),
                &expense_ids,
            )?;
            refers(
                "automation rule",
                r.id,
                "account",
                r.filter_account_id,
                &account_ids,
            )?;
        }

        unique("seen delivery", self.seen_deliveries.iter().map(|d| d.id))?;
        for d in &self.seen_deliveries {
            refers("seen delivery", d.id, "user", Some(d.user_id), &user_ids)?;
            refers(
                "seen delivery",
                d.id,
                "account",
                Some(d.account_id),
                &account_ids,
            )?;
            refers("seen delivery", d.id, "expense", d.expense_id, &expense_ids)?;
        }

        // target_id is not checked: a change outlives the expense it describes
        unique("expense change", self.expense_changes.iter().map(|c| c.id))?;
        for c in &self.expense_changes {
            refers("expense change", c.id, "user", c.user_id, &user_ids)?;
        }

        Ok(())
    }

    /// Parents before children. Categories and accounts each go in one statement, because they
    /// refer to each other within the table (a synced pair of accounts names each other) and
    /// those foreign keys are checked at the end of every statement, not at commit.
    async fn insert(&self, conn: &mut AsyncPgConnection) -> QueryResult<()> {
        let users: Vec<_> = self
            .users
            .iter()
            .map(|u| {
                (
                    users::id.eq(u.id),
                    users::name.eq(&u.name),
                    users::full_name.eq(&u.full_name),
                    users::password_hash.eq(NO_PASSWORD),
                    users::seen_at.eq(u.seen_at),
                    users::seen_before.eq(u.seen_before),
                )
            })
            .collect();
        for chunk in users.chunks(ROWS_PER_INSERT) {
            diesel::insert_into(users::table)
                .values(chunk)
                .execute(conn)
                .await?;
        }

        diesel::insert_into(categories::table)
            .values(&self.categories)
            .execute(conn)
            .await?;
        diesel::insert_into(accounts::table)
            .values(&self.accounts)
            .execute(conn)
            .await?;

        let mut rows = Vec::new();
        let mut transactions = Vec::new();
        let mut categorizations = Vec::new();
        for expense in &self.expenses {
            let (row, t, c) = expense.split();
            rows.push(row);
            transactions.extend(t);
            categorizations.extend(c);
        }
        for chunk in rows.chunks(ROWS_PER_INSERT) {
            diesel::insert_into(expenses::table)
                .values(chunk)
                .execute(conn)
                .await?;
        }
        for chunk in transactions.chunks(ROWS_PER_INSERT) {
            diesel::insert_into(expense_transactions::table)
                .values(chunk)
                .execute(conn)
                .await?;
        }
        for chunk in categorizations.chunks(ROWS_PER_INSERT) {
            diesel::insert_into(expense_categorizations::table)
                .values(chunk)
                .execute(conn)
                .await?;
        }

        for chunk in self.balances.chunks(ROWS_PER_INSERT) {
            diesel::insert_into(balances::table)
                .values(chunk)
                .execute(conn)
                .await?;
        }
        for chunk in self.periodic_rules.chunks(ROWS_PER_INSERT) {
            diesel::insert_into(periodic_rules::table)
                .values(chunk)
                .execute(conn)
                .await?;
        }
        for chunk in self.automation_rules.chunks(ROWS_PER_INSERT) {
            diesel::insert_into(automation_rules::table)
                .values(chunk)
                .execute(conn)
                .await?;
        }
        for chunk in self.seen_deliveries.chunks(ROWS_PER_INSERT) {
            diesel::insert_into(seen_deliveries::table)
                .values(chunk)
                .execute(conn)
                .await?;
        }
        for chunk in self.expense_changes.chunks(ROWS_PER_INSERT) {
            diesel::insert_into(expense_changes::table)
                .values(chunk)
                .execute(conn)
                .await?;
        }

        Ok(())
    }
}

/// Every expense as the aggregate, with three queries rather than three per expense.
async fn load_expenses(conn: &mut AsyncPgConnection) -> QueryResult<Vec<Expense>> {
    let rows: Vec<ExpenseRow> = expenses::table
        .order_by(expenses::id)
        .select(ExpenseRow::as_select())
        .load(conn)
        .await?;

    let mut transactions: HashMap<Uuid, Vec<ExpenseTransactionRow>> = HashMap::new();
    for t in expense_transactions::table
        .order_by((expense_transactions::expense_id, expense_transactions::idx))
        .select(ExpenseTransactionRow::as_select())
        .load::<ExpenseTransactionRow>(conn)
        .await?
    {
        transactions.entry(t.expense_id).or_default().push(t);
    }

    let mut categorizations: HashMap<Uuid, Vec<ExpenseCategorizationRow>> = HashMap::new();
    for c in expense_categorizations::table
        .order_by((
            expense_categorizations::expense_id,
            expense_categorizations::category_id,
        ))
        .select(ExpenseCategorizationRow::as_select())
        .load::<ExpenseCategorizationRow>(conn)
        .await?
    {
        categorizations.entry(c.expense_id).or_default().push(c);
    }

    Ok(rows
        .into_iter()
        .map(|row| {
            let id = row.id;
            Expense::join(
                row,
                transactions.remove(&id).unwrap_or_default(),
                categorizations.remove(&id).unwrap_or_default(),
            )
        })
        .collect())
}

/// Every other table hangs off a user or an expense through a non-null foreign key, so these
/// four being empty means the whole database is.
async fn holds_user_data(conn: &mut AsyncPgConnection) -> QueryResult<bool> {
    diesel::select(
        exists(users::table.select(users::id))
            .or(exists(categories::table.select(categories::id)))
            .or(exists(expenses::table.select(expenses::id)))
            .or(exists(expense_changes::table.select(expense_changes::id))),
    )
    .get_result(conn)
    .await
}

fn unique(kind: &str, ids: impl Iterator<Item = Uuid>) -> Result<HashSet<Uuid>, String> {
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(format!("{kind} id {id} appears twice"));
        }
    }
    Ok(seen)
}

fn refers(
    kind: &str,
    id: Uuid,
    field: &str,
    target: Option<Uuid>,
    known: &HashSet<Uuid>,
) -> Result<(), String> {
    match target {
        Some(target) if !known.contains(&target) => Err(format!(
            "{kind} {id} refers to {field} {target}, which is not in the backup"
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const RUDOLPH: &str = "00000000-0000-7000-8000-000000000001";
    const GIRO: &str = "00000000-0000-7000-8000-0000000000a1";
    const FOOD: &str = "00000000-0000-7000-8000-0000000000c1";
    const SHOPPING: &str = "00000000-0000-7000-8000-0000000000e1";
    const NOWHERE: &str = "00000000-0000-7000-8000-0000000000ff";
    const AT: &str = "2030-02-01T12:00:00Z";

    /// A small backup that passes the check; each test breaks one thing in it.
    fn valid() -> Value {
        json!({
            "users": [
                {"id": RUDOLPH, "name": "rudolph", "fullName": "Rudolph", "seenAt": null,
                 "seenBefore": null}
            ],
            "categories": [
                {"id": FOOD, "name": "Lebensmittel", "description": "", "color": null,
                 "parentId": null, "monthlyLimit": null}
            ],
            "accounts": [
                {"id": GIRO, "userId": RUDOLPH, "name": "Girokonto", "description": "",
                 "color": null, "iban": "DE00000000000000000000", "kind": "real_estate",
                 "availability": "immediately", "risk": "none", "opened": AT, "closed": null,
                 "ownershipFraction": 1.0, "syncAccountId": null, "syncInvert": null,
                 "renditierAccountId": null}
            ],
            "expenses": [
                {"id": SHOPPING, "title": "Einkauf", "store": "REWE", "description": "",
                 "comments": "", "referenceAmount": null, "accrualStart": AT, "accrualEnd": AT,
                 "isTemplate": false, "isTaxRelevant": false, "isUnchecked": false,
                 "isIncomplete": false, "isExtraordinary": false, "receipts": [],
                 "categories": [{"categoryId": FOOD, "weight": 1.0}],
                 "transactions": [
                     {"accountId": GIRO, "date": AT, "bookingDate": null, "amount": -1000,
                      "fraction": null, "comments": "", "statement": ""}
                 ],
                 "positions": [], "createdAt": AT, "updatedAt": AT}
            ],
            "balances": [],
            "periodicRules": [
                {"id": "00000000-0000-7000-8000-0000000000f1", "userId": RUDOLPH,
                 "title": "Einkauf", "templateId": SHOPPING, "period": "monthly",
                 "titleReplacement": null, "lastCreation": AT, "createdAt": AT, "updatedAt": AT}
            ],
            "automationRules": [],
            "seenDeliveries": [],
            // describing an expense that is gone, which a change is allowed to outlive
            "expenseChanges": [
                {"id": "00000000-0000-7000-8000-000000000ac1", "targetId": NOWHERE,
                 "userId": RUDOLPH, "date": AT, "tool": null, "automatic": false,
                 "confidence": 0.0, "oldState": {"id": NOWHERE}, "newState": null}
            ]
        })
    }

    type Breakage = fn(&mut Value);

    fn check(backup: Value) -> Result<(), String> {
        serde_json::from_value::<Backup>(backup).unwrap().check()
    }

    #[test]
    fn a_consistent_backup_passes() {
        assert_eq!(check(valid()), Ok(()));
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let mut backup = valid();
        let account = backup["accounts"][0].clone();
        backup["accounts"].as_array_mut().unwrap().push(account);
        assert_eq!(
            check(backup),
            Err(format!("account id {GIRO} appears twice"))
        );
    }

    #[test]
    fn duplicate_user_names_are_rejected() {
        let mut backup = valid();
        let mut twin = backup["users"][0].clone();
        twin["id"] = json!("00000000-0000-7000-8000-000000000002");
        backup["users"].as_array_mut().unwrap().push(twin);
        assert_eq!(
            check(backup),
            Err("user name 'rudolph' appears twice".into())
        );
    }

    #[test]
    fn dangling_references_are_rejected() {
        let cases: [(&str, Breakage); 5] = [
            ("account", |b| b["accounts"][0]["userId"] = json!(NOWHERE)),
            ("synced account", |b| {
                b["accounts"][0]["syncAccountId"] = json!(NOWHERE);
                b["accounts"][0]["syncInvert"] = json!(false);
            }),
            ("expense account", |b| {
                b["expenses"][0]["transactions"][0]["accountId"] = json!(NOWHERE)
            }),
            ("expense category", |b| {
                b["expenses"][0]["categories"][0]["categoryId"] = json!(NOWHERE)
            }),
            ("rule template", |b| {
                b["periodicRules"][0]["templateId"] = json!(NOWHERE)
            }),
        ];

        for (case, breaks) in cases {
            let mut backup = valid();
            breaks(&mut backup);
            let error = check(backup).expect_err(case);
            assert!(
                error.ends_with(&format!("{NOWHERE}, which is not in the backup")),
                "{case}: {error}"
            );
        }
    }

    #[test]
    fn an_expense_without_transactions_is_rejected() {
        let mut backup = valid();
        backup["expenses"][0]["transactions"] = json!([]);
        assert_eq!(
            check(backup),
            Err(format!(
                "expense {SHOPPING}: Expense must have at least one transaction"
            ))
        );
    }

    #[test]
    fn credentials_cannot_ride_along() {
        let mut backup = valid();
        backup["users"][0]["passwordHash"] = json!("$argon2id$v=19$invented");
        let error = serde_json::from_value::<Backup>(backup).unwrap_err();
        assert!(error.to_string().contains("unknown field `passwordHash`"));
    }

    #[test]
    fn values_are_spelled_as_the_database_spells_them() {
        let backup: Backup = serde_json::from_value(valid()).unwrap();
        let written = serde_json::to_value(&backup).unwrap();
        assert_eq!(written["accounts"][0]["kind"], "real_estate");
        assert_eq!(written["periodicRules"][0]["period"], "monthly");

        let mut unknown = valid();
        unknown["accounts"][0]["kind"] = json!("RealEstate");
        let error = serde_json::from_value::<Backup>(unknown).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("unknown AccountKind: RealEstate")
        );
    }
}
