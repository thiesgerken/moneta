use uuid::Uuid;
use std::error::Error;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use juniper::{GraphQLInputObject, GraphQLObject};
use serde::{Deserialize, Serialize};

use super::Json;

/// An expense with everything that belongs to it in one value.
///
/// The tables store it across three relations, because the analysis groups by account, date
/// and category. This shape is what the changelog remembers and what the importer builds;
/// [`ExpenseRow`] and friends are what actually goes to the database.
#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[serde(rename_all = "camelCase")]
pub struct Expense {
    pub id: Uuid,

    pub title: String,
    pub store: String,
    pub description: String,
    pub comments: String,

    /// reference for fractional transactions, can be None if only fixed transactions are used
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

    pub receipts: Vec<ExpenseReceipt>,
    pub categories: Vec<ExpenseCategorization>,
    pub transactions: Vec<ExpenseTransaction>, // must not be empty
    pub positions: Vec<ExpensePosition>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Expense {
    pub fn validate(&self) -> Result<(), Box<dyn Error>> {
        // booking_end >= booking_start, positive weights and quantities, the amount-xor-fraction
        // rule and non-zero fractions are CHECK constraints now. What is left here is what a
        // CHECK cannot see: facts about the expense as a whole rather than about one row.
        if self.transactions.is_empty() {
            return Err("Expense must have at least one transaction".into());
        }

        if self.transactions.iter().any(|p| p.fraction.is_some()) && self.reference_amount.is_none()
        {
            return Err(
                "Expenses with at least one fractional transaction must contain a reference amount"
                    .into(),
            );
        }

        Ok(())
    }

    /// Splits the aggregate into the rows the tables hold. Transaction order becomes `idx`,
    /// which is what the views sort by when they put the list back together.
    pub fn split(
        &self,
    ) -> (
        ExpenseRow,
        Vec<ExpenseTransactionRow>,
        Vec<ExpenseCategorizationRow>,
    ) {
        let row = ExpenseRow {
            id: self.id,
            title: self.title.clone(),
            store: self.store.clone(),
            description: self.description.clone(),
            comments: self.comments.clone(),
            reference_amount: self.reference_amount,
            booking_start: self.booking_start,
            booking_end: self.booking_end,
            is_template: self.is_template,
            is_tax_relevant: self.is_tax_relevant,
            is_unchecked: self.is_unchecked,
            is_incomplete: self.is_incomplete,
            is_extraordinary: self.is_extraordinary,
            receipts: Json(self.receipts.clone()),
            positions: Json(self.positions.clone()),
            created_at: self.created_at,
            updated_at: self.updated_at,
        };

        let transactions = self
            .transactions
            .iter()
            .enumerate()
            .map(|(idx, t)| ExpenseTransactionRow {
                id: Uuid::now_v7(),
                expense_id: self.id,
                account_id: t.account_id,
                idx: idx as i32,
                date: t.date,
                amount: t.amount,
                fraction: t.fraction,
                comments: t.comments.clone(),
                statement: t.statement.clone(),
            })
            .collect();

        let categories = self
            .categories
            .iter()
            .map(|c| ExpenseCategorizationRow {
                expense_id: self.id,
                category_id: c.category_id,
                weight: c.weight,
            })
            .collect();

        (row, transactions, categories)
    }

    /// Inverse of [`split`](Self::split). The transactions have to arrive in `idx` order, since
    /// the aggregate carries their order in the list rather than in a field.
    pub fn join(
        row: ExpenseRow,
        transactions: Vec<ExpenseTransactionRow>,
        categories: Vec<ExpenseCategorizationRow>,
    ) -> Self {
        Self {
            id: row.id,
            title: row.title,
            store: row.store,
            description: row.description,
            comments: row.comments,
            reference_amount: row.reference_amount,
            booking_start: row.booking_start,
            booking_end: row.booking_end,
            is_template: row.is_template,
            is_tax_relevant: row.is_tax_relevant,
            is_unchecked: row.is_unchecked,
            is_incomplete: row.is_incomplete,
            is_extraordinary: row.is_extraordinary,
            receipts: row.receipts.0,
            positions: row.positions.0,
            transactions: transactions
                .into_iter()
                .map(|t| ExpenseTransaction {
                    account_id: t.account_id,
                    date: t.date,
                    amount: t.amount,
                    fraction: t.fraction,
                    comments: t.comments,
                    statement: t.statement,
                })
                .collect(),
            categories: categories
                .into_iter()
                .map(|c| ExpenseCategorization {
                    category_id: c.category_id,
                    weight: c.weight,
                })
                .collect(),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::expenses, check_for_backend(diesel::pg::Pg))]
pub struct ExpenseRow {
    pub id: Uuid,

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

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::expense_transactions, check_for_backend(diesel::pg::Pg))]
pub struct ExpenseTransactionRow {
    pub id: Uuid,
    pub expense_id: Uuid,
    pub account_id: Uuid,

    /// position in the expense's transaction list, which the UI still shows in order
    pub idx: i32,

    pub date: DateTime<Utc>,
    pub amount: Option<i32>,
    pub fraction: Option<f64>,
    pub comments: String,
    pub statement: String,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::expense_categorizations, check_for_backend(diesel::pg::Pg))]
pub struct ExpenseCategorizationRow {
    pub expense_id: Uuid,
    pub category_id: Uuid,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseTransaction {
    pub account_id: Uuid,
    pub date: DateTime<Utc>,

    // constraint: amount is null OR fraction is null
    // constraint: fraction can only be used if the expense has a reference amount
    // convention: amounts are _added_ to account balances, i.e. a negative amount is a expense
    // convention: fractions are calculated relative to the expense's reference amount
    pub amount: Option<i32>,
    pub fraction: Option<f64>,

    pub comments: String,
    pub statement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseCategorization {
    pub category_id: Uuid,
    // constraint: weight > 0
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[serde(rename_all = "camelCase")]
pub struct ExpensePosition {
    pub amount: Option<i32>,   // total amount of the position
    pub quantity: Option<f64>, // how many things
    pub unit: Option<String>,  // how is "quantity" measured?
    pub title: String,
    pub comments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseReceipt {
    pub uri: String,
    /// can be a local file or e.g. a reference to a document in a dms
    pub ocr: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLInputObject)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseTransactionInput {
    pub account_id: Uuid,
    pub date: DateTime<Utc>,

    pub amount: Option<i32>,
    pub fraction: Option<f64>,

    pub comments: String,
    pub statement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLInputObject)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseCategorizationInput {
    pub category_id: Uuid,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLInputObject)]
#[serde(rename_all = "camelCase")]
pub struct ExpensePositionInput {
    pub amount: Option<i32>,
    pub quantity: Option<f64>,
    pub unit: Option<String>,
    pub title: String,
    pub comments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, GraphQLInputObject)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseReceiptInput {
    pub uri: String,
    pub ocr: Option<String>,
}

impl ExpenseTransactionInput {
    pub fn upgrade(self) -> ExpenseTransaction {
        ExpenseTransaction {
            account_id: self.account_id,
            date: self.date,
            amount: self.amount,
            fraction: self.fraction,
            comments: self.comments,
            statement: self.statement,
        }
    }
}

impl ExpenseCategorizationInput {
    pub fn upgrade(self) -> ExpenseCategorization {
        ExpenseCategorization {
            category_id: self.category_id,
            weight: self.weight,
        }
    }
}

impl ExpensePositionInput {
    pub fn upgrade(self) -> ExpensePosition {
        ExpensePosition {
            amount: self.amount,
            quantity: self.quantity,
            unit: self.unit,
            title: self.title,
            comments: self.comments,
        }
    }
}

impl ExpenseReceiptInput {
    pub fn upgrade(self) -> ExpenseReceipt {
        ExpenseReceipt {
            uri: self.uri,
            ocr: self.ocr,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 12, 0, 0).unwrap()
    }

    fn expense(transactions: Vec<ExpenseTransaction>, reference_amount: Option<i32>) -> Expense {
        Expense {
            id: Uuid::now_v7(),
            title: "Jahresbeitrag".into(),
            store: String::new(),
            description: String::new(),
            comments: String::new(),
            reference_amount,
            booking_start: at(2026, 1, 1),
            booking_end: at(2026, 1, 1),
            is_template: false,
            is_tax_relevant: false,
            is_unchecked: false,
            is_incomplete: false,
            is_extraordinary: false,
            receipts: vec![],
            categories: vec![],
            transactions,
            positions: vec![],
            created_at: at(2026, 1, 1),
            updated_at: at(2026, 1, 1),
        }
    }

    fn transaction(amount: Option<i32>, fraction: Option<f64>) -> ExpenseTransaction {
        ExpenseTransaction {
            account_id: Uuid::now_v7(),
            date: at(2026, 1, 1),
            amount,
            fraction,
            comments: String::new(),
            statement: String::new(),
        }
    }

    #[test]
    fn an_expense_needs_a_transaction() {
        // no CHECK can see this: it is a fact about the rows of another table
        let err = expense(vec![], None)
            .validate()
            .expect_err("an expense without transactions must be rejected");
        assert!(err.to_string().contains("at least one transaction"), "{err}");
    }

    #[test]
    fn a_fractional_transaction_needs_something_to_resolve_against() {
        // the view renders this as null rather than guessing, so it has to be caught on write
        let err = expense(vec![transaction(None, Some(0.25))], None)
            .validate()
            .expect_err("a fraction without a reference amount must be rejected");
        assert!(err.to_string().contains("reference amount"), "{err}");
    }

    #[test]
    fn a_fractional_transaction_with_a_reference_amount_is_fine() {
        assert!(
            expense(vec![transaction(None, Some(0.25))], Some(-2000))
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn splitting_numbers_the_transactions_in_order() {
        let e = expense(
            vec![transaction(Some(-1000), None), transaction(Some(-500), None)],
            None,
        );
        let (row, transactions, _) = e.split();

        assert_eq!(row.id, e.id);
        assert_eq!(
            transactions.iter().map(|t| t.idx).collect::<Vec<_>>(),
            [0, 1]
        );
        assert_eq!(
            transactions.iter().map(|t| t.amount).collect::<Vec<_>>(),
            [Some(-1000), Some(-500)]
        );
        assert!(transactions.iter().all(|t| t.expense_id == e.id));
    }

    #[test]
    fn splitting_and_rejoining_gives_back_the_same_expense() {
        let mut e = expense(
            vec![
                transaction(Some(-1000), None),
                transaction(None, Some(0.5)),
                transaction(Some(-250), None),
            ],
            Some(-2000),
        );
        e.categories = vec![ExpenseCategorization {
            category_id: Uuid::now_v7(),
            weight: 0.25,
        }];
        e.positions = vec![ExpensePosition {
            amount: Some(-500),
            quantity: Some(2.0),
            unit: Some("kg".into()),
            title: "Kartoffeln".into(),
            comments: String::new(),
        }];

        let (row, transactions, categories) = e.split();
        let rejoined = Expense::join(row, transactions, categories);

        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            serde_json::to_value(&rejoined).unwrap()
        );
    }

    #[test]
    fn splitting_carries_the_categorizations() {
        let mut e = expense(vec![transaction(Some(-100), None)], None);
        let category = Uuid::now_v7();
        e.categories = vec![ExpenseCategorization {
            category_id: category,
            weight: 0.75,
        }];

        let (_, _, categories) = e.split();
        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0].category_id, category);
        assert_eq!(categories[0].weight, 0.75);
        assert_eq!(categories[0].expense_id, e.id);
    }
}
