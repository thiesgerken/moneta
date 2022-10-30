use std::error::Error;

use crate::graphql::accounts::GraphQLAccount;
use crate::graphql::categories::GraphQLCategory;
use crate::graphql::root::require_user;
use crate::graphql::root::Context;
use crate::graphql::util::{self};
use crate::models::rendering::RenderedExpenseTransaction;
use crate::models::rendering::{
    RenderedExpense, RenderedExpenseCreateInput, RenderedExpenseInput, RenderedExpenseUpdateInput,
};
use crate::models::ExpenseCategorization;
use crate::models::ExpensePosition;
use crate::models::ExpenseReceipt;
use crate::models::{Account, Expense, ObjectIdWrapper};
use crate::queries;

use chrono::DateTime;
use chrono::Utc;
use juniper::graphql_object;
use juniper::FieldResult;
use juniper::GraphQLObject;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{self, doc};

#[derive(Debug, Clone)]
pub struct GraphQLExpenseCategorization {
    data: ExpenseCategorization,
}

impl GraphQLExpenseCategorization {
    pub fn new(data: ExpenseCategorization) -> Self {
        Self { data }
    }
}

#[graphql_object(Context = Context, name = "ExpenseCategorization")]
impl GraphQLExpenseCategorization {
    async fn category(&self, context: &Context) -> FieldResult<GraphQLCategory> {
        Ok(GraphQLCategory::new(
            context
                .category_loader
                .try_load(self.data.category_id)
                .await?,
        ))
    }

    fn weight(&self) -> f64 {
        self.data.weight
    }
}

#[derive(Debug, Clone)]
pub struct GraphQLExpenseTransaction {
    data: RenderedExpenseTransaction,
}

impl GraphQLExpenseTransaction {
    pub fn new(data: RenderedExpenseTransaction) -> Self {
        GraphQLExpenseTransaction { data }
    }
}

#[graphql_object(Context = Context, name = "ExpenseTransaction")]
impl GraphQLExpenseTransaction {
    async fn account(&self, context: &Context) -> FieldResult<GraphQLAccount> {
        Ok(GraphQLAccount::new(
            context
                .account_loader
                .try_load(self.data.account_id)
                .await?,
        ))
    }

    fn date(&self) -> DateTime<Utc> {
        self.data.date
    }

    fn amount(&self) -> Option<i32> {
        self.data.amount
    }

    fn fraction(&self) -> Option<f64> {
        self.data.fraction
    }

    fn comments(&self) -> &str {
        &self.data.comments
    }

    fn statement(&self) -> &str {
        &self.data.statement
    }

    fn calculated_amount(&self) -> f64 {
        self.data.calculated_amount
    }

    fn effective_amount(&self) -> f64 {
        self.data.effective_amount
    }
}

#[derive(Debug, Clone)]
pub struct GraphQLExpense {
    data: RenderedExpense,
}

impl GraphQLExpense {
    pub fn new(data: RenderedExpense) -> Self {
        GraphQLExpense { data }
    }
}

#[graphql_object(
    Context = Context,name = "Expense")]
impl GraphQLExpense {
    fn id(&self) -> ObjectIdWrapper {
        self.data.id
    }

    fn upstream_id(&self) -> ObjectIdWrapper {
        self.data.upstream_id
    }

    fn title(&self) -> &str {
        &self.data.title
    }

    fn store(&self) -> &str {
        &self.data.store
    }

    fn description(&self) -> &str {
        &self.data.description
    }

    fn comments(&self) -> &str {
        &self.data.comments
    }

    fn reference_amount(&self) -> Option<i32> {
        self.data.reference_amount
    }

    fn booking_start(&self) -> DateTime<Utc> {
        self.data.booking_start
    }

    fn booking_end(&self) -> DateTime<Utc> {
        self.data.booking_end
    }

    // TODO: recurring expense connection (expense loader?)

    fn is_template(&self) -> bool {
        self.data.is_template
    }

    fn is_tax_relevant(&self) -> bool {
        self.data.is_tax_relevant
    }

    fn is_unchecked(&self) -> bool {
        self.data.is_unchecked
    }

    fn is_incomplete(&self) -> bool {
        self.data.is_incomplete
    }

    fn receipts(&self) -> &Vec<ExpenseReceipt> {
        &self.data.receipts
    }

    fn categories(&self) -> Vec<GraphQLExpenseCategorization> {
        self.data
            .categories
            .iter()
            .cloned()
            .map(GraphQLExpenseCategorization::new)
            .collect()
    }

    fn transactions(&self) -> Vec<GraphQLExpenseTransaction> {
        self.data
            .transactions
            .iter()
            .cloned()
            .map(GraphQLExpenseTransaction::new)
            .collect()
    }

    fn positions(&self) -> &Vec<ExpensePosition> {
        &self.data.positions
    }

    fn effective_amount(&self) -> f64 {
        self.data.effective_amount
    }
}

#[derive(GraphQLObject)]
#[graphql(context=Context)]
pub struct ExpenseConnection {
    pub total_count: i32,
    pub edges: Vec<ExpenseNode>,
}

#[derive(GraphQLObject)]
#[graphql(context=Context)]
pub struct ExpenseNode {
    pub node: GraphQLExpense,
}

pub struct ExpenseQuery;

#[graphql_object(
    Context = Context,
)]
impl ExpenseQuery {
    pub async fn list(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
        filter: Option<String>,
        sort: Option<String>,
    ) -> juniper::FieldResult<ExpenseConnection> {
        let uid = require_user(context).await?;

        let (total_count, edges) = util::list_query::<RenderedExpense>(
            "expenses_rendered",
            &context.connection,
            limit,
            skip,
            filter,
            sort,
            Some(doc!("userId": uid)),
        )
        .await?;

        Ok(ExpenseConnection {
            total_count: total_count as i32,
            edges: edges
                .into_iter()
                .map(|e| ExpenseNode {
                    node: GraphQLExpense::new(e),
                })
                .collect(),
        })
    }

    pub async fn get(
        context: &Context,
        id: ObjectIdWrapper,
    ) -> juniper::FieldResult<GraphQLExpense> {
        let uid = require_user(context).await?;
        let collection = context
            .connection
            .collection::<RenderedExpense>("expenses_rendered");

        Ok(GraphQLExpense::new(
            collection
                .find_one(doc! {"_id":*id, "userId": uid}, None)
                .await?
                .ok_or_else(|| format!("Expense {id:?} not found"))?,
        ))
    }
}
pub struct ExpenseMutation;

#[graphql_object(
    Context = Context,
)]
impl ExpenseMutation {
    pub async fn delete(context: &Context, id: ObjectIdWrapper) -> juniper::FieldResult<bool> {
        let user_id = require_user(context).await?;
        let collection = context
            .connection
            .collection::<RenderedExpense>("expenses_rendered");

        // check if there is a rendered expense with the given upstream id belonging to this user
        let original_entry = collection
            .find_one(doc! {"_id":*id, "userId": user_id}, None)
            .await?
            .ok_or("entry to delete not found")?;

        let upstream_collection = context.connection.collection::<Expense>("expenses");
        let upstream_entry = upstream_collection
            .find_one(doc! {"_id":*original_entry.upstream_id}, None)
            .await?
            .unwrap_or_else(|| {
                panic!("cannot find upstream expense for rendered expense {:?}", id)
            });

        // remove rendered expense first ...
        upstream_entry.remove_rendered(&context.connection).await?;
        // ... and then the upstream expense itself
        let count = upstream_collection
            .delete_one(doc! {"_id":*original_entry.upstream_id}, None)
            .await?
            .deleted_count;
        Ok(count == 1)
    }

    pub async fn create(
        context: &Context,
        expense: RenderedExpenseCreateInput,
    ) -> juniper::FieldResult<RenderedExpense> {
        let user_id = require_user(context).await?;
        let collection = context.connection.collection::<Expense>("expenses");

        let occurring_account_ids: Vec<_> =
            expense.transactions.iter().map(|t| *t.account_id).collect();
        let related_accounts =
            queries::query_associated_accounts(&context.connection, &occurring_account_ids).await?;
        let entry = validate_and_unrender(user_id, expense, &related_accounts).await?;

        collection.insert_one(&entry, None).await?;

        let expense = render(user_id, &context.connection, entry, &related_accounts).await?;
        Ok(expense)
    }

    pub async fn update(
        context: &Context,
        id: ObjectIdWrapper,
        expense: RenderedExpenseUpdateInput,
    ) -> juniper::FieldResult<RenderedExpense> {
        let user_id = require_user(context).await?;
        let collection = context
            .connection
            .collection::<RenderedExpense>("expenses_rendered");

        let original_rendered_expense = collection
            .find_one(doc! {"_id":*id, "userId": user_id}, None)
            .await?
            .ok_or("entry to update not found")?;

        let occurring_account_ids: Vec<_> =
            expense.transactions.iter().map(|t| *t.account_id).collect();
        let related_accounts =
            queries::query_associated_accounts(&context.connection, &occurring_account_ids).await?;

        let mut updated_upstream_entry =
            validate_and_unrender(user_id, expense, &related_accounts).await?;

        // find the original upstream expense
        let upstream_collection = context.connection.collection::<Expense>("expenses");
        let upstream_entry = upstream_collection
            .find_one(doc!("_id": *original_rendered_expense.upstream_id), None)
            .await?
            .unwrap_or_else(|| {
                panic!("cannot find upstream expense for rendered expense {:?}", id)
            });

        // delete its rendered expenses
        upstream_entry.remove_rendered(&context.connection).await?;

        // update the upstream expense with the changes
        updated_upstream_entry.id = upstream_entry.id;
        upstream_collection
            .update_one(
                doc!("_id": &*updated_upstream_entry.id),
                doc! {"$set":  bson::to_bson(&updated_upstream_entry).unwrap()},
                None,
            )
            .await?;

        let expense = render(
            user_id,
            &context.connection,
            updated_upstream_entry,
            &related_accounts,
        )
        .await?;
        Ok(expense)
    }
}

pub async fn validate_and_unrender(
    uid: ObjectId,
    expense: RenderedExpenseInput,
    related_accounts: &[Account],
) -> Result<Expense, Box<dyn Error>> {
    if expense
        .transactions
        .iter()
        .map(|t| *t.account_id)
        .any(|a_id| !related_accounts.iter().any(|a| *a.id == a_id))
    {
        return Err(format!(
            "user {:?} tried to create an expense with unknown account id(s)",
            uid
        )
        .into());
    };

    if related_accounts.iter().all(|a| *a.user_id != uid) {
        return Err(format!(
            "user {:?} tried to create an expense consisting only of accounts that don't belong to her",
            uid
        ).into());
    }

    if expense
        .categories
        .iter()
        .map(|c| c.category_id)
        .any(|a_id| !related_accounts.iter().any(|a| *a.id == *a_id))
    {
        return Err(format!(
            "user {:?} tried to create an expense with unknown category id(s)",
            uid
        )
        .into());
    };

    // cast to normal expense and insert into database
    let entry = Expense::from_rendered(expense);
    entry.validate()?;

    Ok(entry)
}

pub async fn render(
    uid: ObjectId,
    connection: &mongodb::Database,
    entry: Expense,
    related_accounts: &[Account],
) -> Result<RenderedExpense, Box<dyn Error>> {
    // create rendered expenses and insert them into database
    let rendered_expenses = entry.render(related_accounts)?;
    let collection = connection.collection::<RenderedExpense>("expenses_rendered");
    collection.insert_many(&rendered_expenses, None).await?;

    // find the rendered expense that was rendered for this user and return it
    let rendered_entry = rendered_expenses
        .into_iter()
        .find(|b| *b.user_id == uid)
        .unwrap_or_else(|| {
            panic!(
                "rendering did not result in a expense for this user?! expense {:?}",
                entry.id
            )
        });
    Ok(rendered_entry)
}
