use std::collections::HashMap;
use std::error::Error;

use crate::graphql::accounts::GraphQLAccount;
use crate::graphql::categories::GraphQLCategory;
use crate::graphql::root::Context;
use crate::graphql::root::require_user;
use crate::graphql::users::GraphQLUserInfo;
use crate::graphql::util::{self};
use crate::models::Category;
use crate::models::ExpenseCategorization;
use crate::models::ExpenseChangelogEntry;
use crate::models::ExpensePosition;
use crate::models::ExpenseReceipt;
use crate::models::rendering::RenderedExpenseTransaction;
use crate::models::rendering::{
    RenderedExpense, RenderedExpenseCreateInput, RenderedExpenseInput, RenderedExpenseUpdateInput,
};
use crate::models::{Account, Expense, ObjectIdWrapper};
use crate::queries;

use bson::Document;
use chrono::DateTime;
use chrono::Utc;
use futures::TryStreamExt;
use juniper::FieldResult;
use juniper::GraphQLObject;
use juniper::graphql_object;
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

    fn is_extraordinary(&self) -> bool {
        self.data.is_extraordinary
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

    async fn changelog(
        &self,
        context: &Context,
    ) -> juniper::FieldResult<Vec<GraphQLExpenseChangelogEntry>> {
        let collection = context
            .connection
            .collection::<ExpenseChangelogEntry>("expense_changes");

        Ok(collection
            .find(doc! {"targetId": *self.data.upstream_id})
            .await?
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .map(|entry| GraphQLExpenseChangelogEntry::new(entry, self.data.id.into()))
            .collect())
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

#[derive(Debug, Clone)]
pub struct GraphQLExpenseChangelogEntry {
    data: ExpenseChangelogEntry,
    rendered_id: ObjectId,
}

impl GraphQLExpenseChangelogEntry {
    pub fn new(data: ExpenseChangelogEntry, rendered_id: ObjectId) -> Self {
        GraphQLExpenseChangelogEntry { data, rendered_id }
    }
}

#[graphql_object(Context = Context, name = "ExpenseChangelogEntry")]
impl GraphQLExpenseChangelogEntry {
    fn target_id(&self) -> ObjectIdWrapper {
        self.data.target_id
    }
    fn id(&self) -> ObjectIdWrapper {
        self.data.id
    }
    fn expense_id(&self) -> ObjectIdWrapper {
        self.rendered_id.into()
    }
    fn user_id(&self) -> Option<ObjectIdWrapper> {
        self.data.user_id
    }
    fn date(&self) -> DateTime<Utc> {
        self.data.date
    }
    fn tool(&self) -> Option<&str> {
        self.data.tool.as_deref()
    }
    fn automatic(&self) -> bool {
        self.data.automatic
    }
    fn confidence(&self) -> f64 {
        self.data.confidence
    }
    fn old_state(&self) -> Option<Expense> {
        self.data.old_state.clone()
    }
    fn new_state(&self) -> Option<Expense> {
        self.data.new_state.clone()
    }
    fn diff(&self) -> Option<String> {
        let mut old_state = self.data.old_state.clone()?;
        let new_state = self.data.new_state.as_ref()?;

        old_state.updated_at = new_state.updated_at;
        util::json_diff(&old_state, &new_state).ok()
    }
    async fn user(&self, context: &Context) -> FieldResult<Option<GraphQLUserInfo>> {
        let Some(user_id) = self.data.user_id else {
            return Ok(None);
        };

        Ok(Some(GraphQLUserInfo::new(
            context.user_loader.try_load(user_id).await?,
        )))
    }
}

#[derive(GraphQLObject)]
#[graphql(context=Context)]
pub struct ExpenseChangelogConnection {
    pub edges: Vec<ExpenseChangelogNode>,
}

#[derive(GraphQLObject)]
#[graphql(context=Context)]
pub struct ExpenseChangelogNode {
    pub node: GraphQLExpenseChangelogEntry,
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
                .find_one(doc! {"_id":*id, "userId": uid})
                .await?
                .ok_or_else(|| format!("Expense {id:?} not found"))?,
        ))
    }

    pub async fn changelog(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
    ) -> juniper::FieldResult<ExpenseChangelogConnection> {
        let uid = require_user(context).await?;
        let limit = limit.unwrap_or(50);
        let skip = skip.unwrap_or(0);

        // Get the last rendered expenses for the user, sorted by last updated
        let rendered_collection = context
            .connection
            .collection::<RenderedExpense>("expenses_rendered");

        let rendered_expenses = rendered_collection
            .clone_with_type::<Document>()
            .find(doc! {"userId": uid})
            .sort(doc! {"updatedAt": -1})
            .projection(doc! {"_id": 1, "upstreamId": 1, "updatedAt": 1})
            .limit((skip + limit) as i64)
            .await?
            .try_collect::<Vec<Document>>()
            .await?;

        // Extract upstream IDs
        let upstream_ids: Vec<ObjectId> = rendered_expenses
            .iter()
            .map(|expense| expense.get_object_id("upstreamId").unwrap())
            .collect();

        if upstream_ids.is_empty() {
            return Ok(ExpenseChangelogConnection { edges: vec![] });
        }

        let upstream_id_to_rendered_id: HashMap<ObjectId, ObjectId> = rendered_expenses
            .into_iter()
            .map(|expense| {
                (
                    expense.get_object_id("upstreamId").unwrap(),
                    expense.get_object_id("_id").unwrap(),
                )
            })
            .collect();

        // Query changelog entries for these upstream IDs
        let changelog_collection = context
            .connection
            .collection::<ExpenseChangelogEntry>("expense_changes");

        let changelog_entries = changelog_collection
            .find(doc! {"targetId": {"$in": upstream_ids}})
            .sort(doc! {"date": -1})
            .skip(skip as u64)
            .limit(limit as i64)
            .await?
            .try_collect::<Vec<_>>()
            .await?;

        Ok(ExpenseChangelogConnection {
            edges: changelog_entries
                .into_iter()
                .map(|entry| {
                    let rendered_id = upstream_id_to_rendered_id
                        .get(&entry.target_id)
                        .unwrap()
                        .clone();
                    ExpenseChangelogNode {
                        node: GraphQLExpenseChangelogEntry::new(entry, rendered_id),
                    }
                })
                .collect(),
        })
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
            .find_one(doc! {"_id":*id, "userId": user_id})
            .await?
            .ok_or("entry to delete not found")?;

        let upstream_collection = context.connection.collection::<Expense>("expenses");
        let upstream_entry = upstream_collection
            .find_one(doc! {"_id":*original_entry.upstream_id})
            .await?
            .unwrap_or_else(|| {
                panic!("cannot find upstream expense for rendered expense {:?}", id)
            });

        // remove rendered expense first ...
        upstream_entry.remove_rendered(&context.connection).await?;
        // ... and then the upstream expense itself
        let count = upstream_collection
            .delete_one(doc! {"_id":*original_entry.upstream_id})
            .await?
            .deleted_count;

        // create a changelog entry for this removal
        let changelog_entry = ExpenseChangelogEntry {
            id: ObjectId::new().into(),
            user_id: Some(user_id.into()),
            target_id: upstream_entry.id,
            date: Utc::now(),
            old_state: Some(upstream_entry),
            new_state: None,
            tool: None,
            automatic: false,
            confidence: 1.0,
        };
        context
            .connection
            .collection::<ExpenseChangelogEntry>("expense_changes")
            .insert_one(&changelog_entry)
            .await?;

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
        let categories = queries::query_categories(&context.connection).await?;
        let entry = validate_and_unrender(user_id, expense, &related_accounts, &categories).await?;

        collection.insert_one(&entry).await?;

        // create a changelog entry for this creation
        let changelog_entry = ExpenseChangelogEntry {
            id: ObjectId::new().into(),
            user_id: Some(user_id.into()),
            target_id: entry.id,
            date: entry.created_at,
            old_state: None,
            new_state: Some(entry.clone()),
            tool: None,
            automatic: false,
            confidence: 1.0,
        };
        context
            .connection
            .collection::<ExpenseChangelogEntry>("expense_changes")
            .insert_one(&changelog_entry)
            .await?;

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
            .find_one(doc! {"_id":*id, "userId": user_id})
            .await?
            .ok_or("entry to update not found")?;

        let occurring_account_ids: Vec<_> =
            expense.transactions.iter().map(|t| *t.account_id).collect();
        let related_accounts =
            queries::query_associated_accounts(&context.connection, &occurring_account_ids).await?;
        let categories = queries::query_categories(&context.connection).await?;

        let mut updated_upstream_entry =
            validate_and_unrender(user_id, expense, &related_accounts, &categories).await?;

        // find the original upstream expense
        let upstream_collection = context.connection.collection::<Expense>("expenses");
        let upstream_entry = upstream_collection
            .find_one(doc!("_id": *original_rendered_expense.upstream_id))
            .await?
            .unwrap_or_else(|| {
                panic!("cannot find upstream expense for rendered expense {:?}", id)
            });

        // delete its rendered expenses
        upstream_entry.remove_rendered(&context.connection).await?;

        // update the upstream expense with the changes
        updated_upstream_entry.id = upstream_entry.id;
        updated_upstream_entry.updated_at = Utc::now();
        updated_upstream_entry.created_at = upstream_entry.created_at;

        upstream_collection
            .update_one(
                doc!("_id": &*updated_upstream_entry.id),
                doc! {"$set":  bson::serialize_to_bson(&updated_upstream_entry).unwrap()},
            )
            .await?;

        // create a changelog entry for this update
        let changelog_entry = ExpenseChangelogEntry {
            id: ObjectId::new().into(),
            user_id: Some(user_id.into()),
            target_id: upstream_entry.id,
            date: updated_upstream_entry.updated_at,
            old_state: Some(upstream_entry),
            new_state: Some(updated_upstream_entry.clone()),
            tool: None,
            automatic: false,
            confidence: 1.0,
        };
        context
            .connection
            .collection::<ExpenseChangelogEntry>("expense_changes")
            .insert_one(&changelog_entry)
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
    categories: &[Category],
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
        .any(|a_id| !categories.iter().any(|a| *a.id == *a_id))
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
    collection.insert_many(&rendered_expenses).await?;

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
