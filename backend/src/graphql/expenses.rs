use std::collections::HashMap;
use std::error::Error;

use uuid::Uuid;

use crate::graphql::accounts::GraphQLAccount;
use crate::graphql::categories::GraphQLCategory;
use crate::graphql::root::{Context, require_user};
use crate::graphql::scoped;
use crate::graphql::users::GraphQLUserInfo;
use crate::graphql::util::{self, SortDirection};
use crate::models::Category;
use crate::models::ExpenseCategorization;
use crate::models::ExpenseChangelogEntry;
use crate::models::ExpensePosition;
use crate::models::ExpenseReceipt;
use crate::models::Json;
use crate::models::rendering::RenderedExpenseTransaction;
use crate::models::rendering::{
    RenderedExpense, RenderedExpenseCreateInput, RenderedExpenseInput, RenderedExpenseUpdateInput,
};
use crate::models::{Account, AutomationRule, Expense};
use crate::queries;
use crate::rules;
use crate::schema::{expense_categorizations, expense_changes, expenses};
use crate::schema_views::{expense_transactions_rendered, expenses_rendered};

use chrono::DateTime;
use chrono::Utc;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{Bool, Nullable, Text, Timestamptz};
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use juniper::{FieldResult, GraphQLEnum, GraphQLObject, graphql_object};

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

    /// when the money moved (Wertstellung)
    fn date(&self) -> DateTime<Utc> {
        self.data.date
    }

    /// when the bank posted the line; null for anything not delivered by one
    fn booking_date(&self) -> Option<DateTime<Utc>> {
        self.data.booking_date
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

#[graphql_object(Context = Context, name = "Expense")]
impl GraphQLExpense {
    fn id(&self) -> Uuid {
        self.data.id
    }

    fn upstream_id(&self) -> Uuid {
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

    fn accrual_start(&self) -> DateTime<Utc> {
        self.data.accrual_start
    }

    fn accrual_end(&self) -> DateTime<Utc> {
        self.data.accrual_end
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

    async fn changelog(&self, context: &Context) -> FieldResult<Vec<GraphQLExpenseChangelogEntry>> {
        let conn = &mut context.conn().await?;

        Ok(expense_changes::table
            .filter(expense_changes::target_id.eq(self.data.upstream_id))
            .order_by(expense_changes::date.desc())
            .select(ExpenseChangelogEntry::as_select())
            .load(conn)
            .await?
            .into_iter()
            .map(GraphQLExpenseChangelogEntry::new)
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

/// One field a changelog entry changed.
///
/// The values travel as JSON rather than as text the server wrote, because only the reader can
/// say what they mean: the same `-17900` is money on a transaction, a weight on a
/// categorization and an id on an account, and it is the browser that holds the locale, the
/// account names and the formatting the rest of the app uses.
#[derive(GraphQLObject)]
#[graphql(context=Context)]
pub struct ExpenseChange {
    /// `transactions/0/amount`
    pub path: String,
    /// Null where the field is absent on that side, which is how an added or removed list
    /// element arrives: one change carrying the whole element. A field that is present and
    /// null arrives as the JSON string `null`.
    pub old: Option<String>,
    pub new: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GraphQLExpenseChangelogEntry {
    data: ExpenseChangelogEntry,
}

impl GraphQLExpenseChangelogEntry {
    pub fn new(data: ExpenseChangelogEntry) -> Self {
        GraphQLExpenseChangelogEntry { data }
    }
}

#[graphql_object(Context = Context, name = "ExpenseChangelogEntry")]
impl GraphQLExpenseChangelogEntry {
    fn target_id(&self) -> Uuid {
        self.data.target_id
    }
    fn id(&self) -> Uuid {
        self.data.id
    }
    /// The rendering a reader sees carries the upstream id, so this is the same value as
    /// `targetId`; it stays because the frontend keys its changelog rows on it.
    fn expense_id(&self) -> Uuid {
        self.data.target_id
    }
    fn user_id(&self) -> Option<Uuid> {
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
        self.data.old_state.clone().map(|state| state.0)
    }
    fn new_state(&self) -> Option<Expense> {
        self.data.new_state.clone().map(|state| state.0)
    }
    /// What this entry changed, field by field. Empty for a creation, and for the entries an
    /// import writes: the export carries the expense as it is now rather than as it was, so
    /// `import.rs` stores the same state on both sides.
    fn changes(&self) -> Vec<ExpenseChange> {
        let (Some(old_state), Some(new_state)) =
            (self.data.old_state.clone(), self.data.new_state.as_ref())
        else {
            return vec![];
        };

        let mut old_state = old_state.0;
        // moves on every save and says nothing about what the save did
        old_state.updated_at = new_state.0.updated_at;

        let (Ok(old), Ok(new)) = (
            serde_json::to_value(&old_state),
            serde_json::to_value(&new_state.0),
        ) else {
            return vec![];
        };

        util::json_changes(&old, &new)
            .into_iter()
            .map(|change| ExpenseChange {
                path: change.path,
                old: change.old.map(|v| v.to_string()),
                new: change.new.map(|v| v.to_string()),
            })
            .collect()
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
    /// how many entries match, which for a caller that only wants the number is the whole
    /// answer: "3 Änderungen seit gestern" needs no entry to be read
    pub total_count: i32,
    pub edges: Vec<ExpenseChangelogNode>,
}

#[derive(GraphQLObject)]
#[graphql(context=Context)]
pub struct ExpenseChangelogNode {
    pub node: GraphQLExpenseChangelogEntry,
}

/// Whose changes the activity feed shows. Filtered here rather than in the reader, which only
/// ever holds a page: dropping a page's own entries would leave a handful of rows and call it
/// everyone else's activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, GraphQLEnum)]
pub enum ChangelogAuthor {
    #[default]
    Anyone,
    Me,
    Others,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, GraphQLEnum)]
pub enum ExpenseSortField {
    #[default]
    AccrualStart,
    /// earliest Wertstellung among the transactions
    ValueDate,
    /// earliest Buchungstag among the transactions that have one
    BookingDate,
    Title,
    Store,
    Description,
    Comments,
    EffectiveAmount,
    /// template first, then the entries still needing attention
    Status,
    Accounts,
    Categories,
}

/// The flag checkboxes above the expense table. Each one being off means "hide the expenses
/// that carry this flag"; `normal` off means "show only expenses that carry some flag".
#[derive(Debug, Clone, Copy, juniper::GraphQLInputObject)]
pub struct ExpenseFlagFilter {
    pub templates: bool,
    pub tax_relevant: bool,
    pub extraordinary: bool,
    pub unchecked: bool,
    pub incomplete: bool,
    pub normal: bool,
}

impl Default for ExpenseFlagFilter {
    fn default() -> Self {
        Self {
            templates: true,
            tax_relevant: true,
            extraordinary: true,
            unchecked: true,
            incomplete: true,
            normal: true,
        }
    }
}

type ExpensePredicate =
    Box<dyn BoxableExpression<expenses_rendered::table, diesel::pg::Pg, SqlType = Bool>>; // scoped-ok: names the table as a type, builds no query

/// Everything a search term can match: the free-text fields, the position titles inside the
/// jsonb, and -- if the term reads as a number -- the total in either sign, since the table
/// shows expenses and income in the same column.
fn search_predicate(term: &str) -> ExpensePredicate {
    let pattern = util::ilike_pattern(term);

    let mut predicate: ExpensePredicate = Box::new(
        expenses_rendered::title
            .ilike(pattern.clone())
            .or(expenses_rendered::store.ilike(pattern.clone()))
            .or(expenses_rendered::description.ilike(pattern.clone()))
            .or(expenses_rendered::comments.ilike(pattern.clone()))
            .or(sql::<Bool>("positions::text ILIKE ").bind::<Text, _>(pattern)),
    );

    if let Ok(value) = term.replace(',', ".").parse::<f64>()
        && value != 0.0
    {
        let lower = (value * 100.0).floor();
        let upper = (value * 100.0).ceil();

        predicate = Box::new(
            predicate
                .or(expenses_rendered::effective_amount.between(lower, upper))
                .or(expenses_rendered::effective_amount.between(-upper, -lower)),
        );
    }

    predicate
}

/// Where a transaction stands on the Buchungstag axis. A string because diesel has no
/// `coalesce`.
const BOOKED_OR_VALUED: &str = "coalesce(booking_date, date)";

pub struct ExpenseQuery;

#[graphql_object(
    Context = Context,
)]
impl ExpenseQuery {
    #[allow(clippy::too_many_arguments)]
    pub async fn list(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
        search: Option<String>,
        ids: Option<Vec<Uuid>>,
        account_ids: Option<Vec<Uuid>>,
        category_ids: Option<Vec<Uuid>>,
        flags: Option<ExpenseFlagFilter>,
        accrued_after: Option<DateTime<Utc>>,
        accrued_before: Option<DateTime<Utc>>,
        valued_after: Option<DateTime<Utc>>,
        valued_before: Option<DateTime<Utc>>,
        booked_after: Option<DateTime<Utc>>,
        booked_before: Option<DateTime<Utc>>,
        sort_by: Option<ExpenseSortField>,
        sort_dir: Option<SortDirection>,
    ) -> FieldResult<ExpenseConnection> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let (limit, offset) = util::page(limit, skip);
        let direction = sort_dir.unwrap_or_default();
        let flags = flags.unwrap_or_default();
        let search = search.filter(|s| !s.trim().is_empty());

        // A boxed query is consumed by the count, so the filter is built twice rather than
        // cloned.
        let filtered = || {
            let mut query = scoped::expenses(user_id);

            if let Some(term) = &search {
                query = query.filter(search_predicate(term.trim()));
            }

            // An empty list is not "everything": a caller that meant to name expenses and
            // named none wants none, which is what the other id filters would say too if they
            // could tell the two apart.
            if let Some(ids) = &ids {
                query = query.filter(expenses_rendered::id.eq_any(ids));
            }

            if let Some(account_ids) = &account_ids
                && !account_ids.is_empty()
            {
                query = query.filter(
                    expenses_rendered::id.eq_any(
                        scoped::expense_transactions(user_id)
                            .filter(expense_transactions_rendered::account_id.eq_any(account_ids))
                            .select(expense_transactions_rendered::expense_id),
                    ),
                );
            }

            if let Some(category_ids) = &category_ids
                && !category_ids.is_empty()
            {
                query = query.filter(
                    expenses_rendered::id.eq_any(
                        expense_categorizations::table
                            .filter(expense_categorizations::category_id.eq_any(category_ids))
                            .select(expense_categorizations::expense_id),
                    ),
                );
            }

            if !flags.templates {
                query = query.filter(expenses_rendered::is_template.eq(false));
            }
            if !flags.tax_relevant {
                query = query.filter(expenses_rendered::is_tax_relevant.eq(false));
            }
            if !flags.extraordinary {
                query = query.filter(expenses_rendered::is_extraordinary.eq(false));
            }
            if !flags.unchecked {
                query = query.filter(expenses_rendered::is_unchecked.eq(false));
            }
            if !flags.incomplete {
                query = query.filter(expenses_rendered::is_incomplete.eq(false));
            }
            if !flags.normal {
                query = query.filter(
                    expenses_rendered::is_template
                        .or(expenses_rendered::is_tax_relevant)
                        .or(expenses_rendered::is_extraordinary)
                        .or(expenses_rendered::is_unchecked)
                        .or(expenses_rendered::is_incomplete),
                );
            }

            // Overlap, not containment: an expense accruing from January to June was a cost in
            // March, and a caller asking for March wants it. Each end also stands on its own --
            // "still accruing after X", "already accruing before Y" -- which is why they are
            // two arguments rather than a window.
            if let Some(after) = accrued_after {
                query = query.filter(expenses_rendered::accrual_end.ge(after));
            }
            if let Some(before) = accrued_before {
                query = query.filter(expenses_rendered::accrual_start.lt(before));
            }

            // The accrual period is when an expense was *for*; these two are when the money
            // actually moved. A prepayment's period spans a year while its payment lands on
            // one day, so a caller asking "what happened in March" wants these.
            if let Some(after) = valued_after {
                query = query.filter(
                    expenses_rendered::id.eq_any(
                        scoped::expense_transactions(user_id)
                            .filter(expense_transactions_rendered::date.ge(after))
                            .select(expense_transactions_rendered::expense_id),
                    ),
                );
            }
            if let Some(before) = valued_before {
                query = query.filter(
                    expenses_rendered::id.eq_any(
                        scoped::expense_transactions(user_id)
                            .filter(expense_transactions_rendered::date.lt(before))
                            .select(expense_transactions_rendered::expense_id),
                    ),
                );
            }

            // A transaction the bank never posted has no booking date and falls back to its
            // own Wertstellung -- the same fallback the column and the sort use, so a range
            // picks out the rows the column shows inside it.
            if let Some(after) = booked_after {
                query = query.filter(
                    expenses_rendered::id.eq_any(
                        scoped::expense_transactions(user_id)
                            .filter(sql::<Timestamptz>(BOOKED_OR_VALUED).ge(after))
                            .select(expense_transactions_rendered::expense_id),
                    ),
                );
            }
            if let Some(before) = booked_before {
                query = query.filter(
                    expenses_rendered::id.eq_any(
                        scoped::expense_transactions(user_id)
                            .filter(sql::<Timestamptz>(BOOKED_OR_VALUED).lt(before))
                            .select(expense_transactions_rendered::expense_id),
                    ),
                );
            }

            query
        };

        let total_count: i64 = filtered().count().get_result(conn).await?;

        let query = filtered();
        let query = match sort_by.unwrap_or_default() {
            ExpenseSortField::AccrualStart => {
                crate::order_by!(query, direction, expenses_rendered::accrual_start)
            }
            // The dates live on the transactions, of which an expense can have several on
            // several days, so the column shows a span and sorts on where it begins. Read back
            // out of the rendered jsonb rather than joining, because that is already the
            // reader's own view of them -- a mirrored transaction included, a stranger's not.
            ExpenseSortField::ValueDate => crate::order_by!(
                query,
                direction,
                sql::<Nullable<Timestamptz>>(
                    "(SELECT min((t->>'date')::timestamptz) FROM jsonb_array_elements(transactions) t)"
                )
            ),
            // Falling back to the transaction's own Wertstellung, because many of them
            // were never posted by a bank and have no booking date at all. Where a bank did
            // post one the two are a day or two apart anyway, so it is the closest thing to an
            // answer, and it keeps the column, the sort and the period filter agreeing about
            // where such a row belongs.
            ExpenseSortField::BookingDate => crate::order_by!(
                query,
                direction,
                sql::<Timestamptz>(
                    "(SELECT min(coalesce((t->>'bookingDate')::timestamptz, (t->>'date')::timestamptz)) \
                     FROM jsonb_array_elements(transactions) t)"
                )
            ),
            ExpenseSortField::Title => crate::order_by!(query, direction, expenses_rendered::title),
            ExpenseSortField::Store => crate::order_by!(query, direction, expenses_rendered::store),
            ExpenseSortField::Description => {
                crate::order_by!(query, direction, expenses_rendered::description)
            }
            ExpenseSortField::Comments => {
                crate::order_by!(query, direction, expenses_rendered::comments)
            }
            ExpenseSortField::EffectiveAmount => {
                crate::order_by!(query, direction, expenses_rendered::effective_amount)
            }
            // the lists are aggregates; the first entry is what the column shows first
            ExpenseSortField::Accounts => crate::order_by!(
                query,
                direction,
                sql::<Text>("transactions->0->>'accountId'")
            ),
            ExpenseSortField::Categories => crate::order_by!(
                query,
                direction,
                sql::<Text>("categories->0->>'categoryId'")
            ),
            // the default the table opens with: what needs attention first, then newest.
            // Without the booking date as a secondary key the id tiebreaker would order by
            // insertion, which for an imported history is meaningless.
            ExpenseSortField::Status => match direction {
                SortDirection::Asc => query.order_by((
                    expenses_rendered::is_template.desc(),
                    expenses_rendered::is_incomplete.asc(),
                    expenses_rendered::is_unchecked.asc(),
                    expenses_rendered::accrual_start.desc(),
                )),
                SortDirection::Desc => query.order_by((
                    expenses_rendered::is_template.asc(),
                    expenses_rendered::is_incomplete.desc(),
                    expenses_rendered::is_unchecked.desc(),
                    expenses_rendered::accrual_start.desc(),
                )),
            },
        };

        // The page is picked and rendered in two steps on purpose. Rendering an expense means
        // aggregating its transactions, categories and positions into jsonb, and a sort the
        // view cannot answer from an index has to render every row it might return before it
        // can order them -- every one of them to hand back fifteen, which is 550 ms of the 560 ms
        // the default sort used to take. Ordering ids alone touches none of that.
        let ids: Vec<Uuid> = query
            // without a tiebreaker, two pages of an equal sort key can repeat or skip rows
            .then_order_by(expenses_rendered::id.asc())
            .limit(limit)
            .offset(offset)
            .select(expenses_rendered::id)
            .load(conn)
            .await?;

        let mut found: Vec<RenderedExpense> = scoped::expenses(user_id)
            .filter(expenses_rendered::id.eq_any(&ids))
            .select(RenderedExpense::as_select())
            .load(conn)
            .await?;

        // `IN` says nothing about order, and the order is the whole point of the first query
        let position: HashMap<Uuid, usize> = ids.iter().copied().zip(0..).collect();
        found.sort_by_key(|e| position.get(&e.id).copied().unwrap_or(usize::MAX));

        Ok(ExpenseConnection {
            total_count: total_count as i32,
            edges: found
                .into_iter()
                .map(|node| ExpenseNode {
                    node: GraphQLExpense::new(node),
                })
                .collect(),
        })
    }

    pub async fn get(context: &Context, id: Uuid) -> FieldResult<GraphQLExpense> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        Ok(GraphQLExpense::new(load_rendered(conn, user_id, id).await?))
    }

    pub async fn changelog(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
        author: Option<ChangelogAuthor>,
        since: Option<DateTime<Utc>>,
    ) -> FieldResult<ExpenseChangelogConnection> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let (limit, offset) = util::page(limit.or(Some(50)), skip);

        // A boxed query is consumed by the count, so the filter is built twice rather than
        // cloned.
        let filtered = || {
            // A change outlives the expense it describes, so this reaches the visible expenses
            // through the view rather than joining: a deleted expense has no rendering left
            // and its changes drop out of the list, which is what the old query did too.
            let mut query = expense_changes::table
                .filter(
                    expense_changes::target_id
                        .eq_any(scoped::expenses(user_id).select(expenses_rendered::upstream_id)),
                )
                .into_boxed();

            // An automation writes the owner's id, so "someone else" means a different person,
            // not a different actor: a rule of mine firing is still mine.
            query = match author.unwrap_or_default() {
                ChangelogAuthor::Anyone => query,
                ChangelogAuthor::Me => query.filter(expense_changes::user_id.eq(user_id)),
                // an entry with no author is nobody's -- an import, which is not someone else
                ChangelogAuthor::Others => query.filter(expense_changes::user_id.ne(user_id)),
            };

            if let Some(since) = since {
                query = query.filter(expense_changes::date.ge(since));
            }

            query
        };

        let total_count: i64 = filtered().count().get_result(conn).await?;

        let entries = filtered()
            .order_by(expense_changes::date.desc())
            .then_order_by(expense_changes::id.desc())
            .limit(limit)
            .offset(offset)
            .select(ExpenseChangelogEntry::as_select())
            .load(conn)
            .await?;

        Ok(ExpenseChangelogConnection {
            total_count: total_count as i32,
            edges: entries
                .into_iter()
                .map(|node| ExpenseChangelogNode {
                    node: GraphQLExpenseChangelogEntry::new(node),
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
    pub async fn delete(context: &Context, id: Uuid) -> FieldResult<bool> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        // the rendering is what the user was shown, so it is what they are allowed to remove
        let rendered = load_rendered(conn, user_id, id).await?;
        let upstream_id = rendered.upstream_id;

        let deleted = conn
            .transaction::<_, diesel::result::Error, _>(async |conn| {
                let Some(old_state) = queries::load_expense(conn, upstream_id).await? else {
                    return Ok(false);
                };

                let count = diesel::delete(expenses::table.find(upstream_id))
                    .execute(conn)
                    .await?;

                diesel::insert_into(expense_changes::table)
                    .values(&ExpenseChangelogEntry {
                        id: Uuid::now_v7(),
                        user_id: Some(user_id),
                        target_id: upstream_id,
                        date: Utc::now(),
                        old_state: Some(Json(old_state)),
                        new_state: None,
                        tool: None,
                        automatic: false,
                        confidence: 1.0,
                    })
                    .execute(conn)
                    .await?;

                Ok(count == 1)
            })
            .await?;

        Ok(deleted)
    }

    pub async fn create(
        context: &Context,
        expense: RenderedExpenseCreateInput,
    ) -> FieldResult<GraphQLExpense> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let entry = validate_and_unrender(conn, user_id, expense).await?;
        let id = entry.id;

        conn.transaction::<_, diesel::result::Error, _>(async |conn| {
            queries::insert_expense(conn, &entry).await?;

            diesel::insert_into(expense_changes::table)
                .values(&ExpenseChangelogEntry {
                    id: Uuid::now_v7(),
                    user_id: Some(user_id),
                    target_id: entry.id,
                    date: entry.created_at,
                    old_state: None,
                    new_state: Some(Json(entry.clone())),
                    tool: None,
                    automatic: false,
                    confidence: 1.0,
                })
                .execute(conn)
                .await?;

            Ok(())
        })
        .await?;

        Ok(GraphQLExpense::new(load_rendered(conn, user_id, id).await?))
    }

    pub async fn update(
        context: &Context,
        id: Uuid,
        expense: RenderedExpenseUpdateInput,
    ) -> FieldResult<GraphQLExpense> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let rendered = load_rendered(conn, user_id, id).await?;
        let upstream_id = rendered.upstream_id;

        let mut updated = validate_and_unrender(conn, user_id, expense).await?;
        updated.id = upstream_id;
        updated.updated_at = Utc::now();

        conn.transaction::<_, Box<dyn Error + Send + Sync>, _>(async |conn| {
            let old_state = queries::load_expense(conn, upstream_id)
                .await?
                .ok_or("entry to update disappeared")?;

            updated.created_at = old_state.created_at;
            queries::replace_expense(conn, &updated).await?;

            diesel::insert_into(expense_changes::table)
                .values(&ExpenseChangelogEntry {
                    id: Uuid::now_v7(),
                    user_id: Some(user_id),
                    target_id: upstream_id,
                    date: updated.updated_at,
                    old_state: Some(Json(old_state)),
                    new_state: Some(Json(updated.clone())),
                    tool: None,
                    automatic: false,
                    confidence: 1.0,
                })
                .execute(conn)
                .await?;

            Ok(())
        })
        .await?;

        Ok(GraphQLExpense::new(
            load_rendered(conn, user_id, upstream_id).await?,
        ))
    }

    /// Runs the automation rules over an expense again, as though its bank line had only just
    /// arrived, and replaces it with what the matching rule produces.
    ///
    /// Everything the rule decides is overwritten -- title, categories, the other side of a
    /// split, the flags. What the bank said stays: the amount, the date and the statement are
    /// imposed back onto the template, exactly as they were on delivery. That is the whole
    /// point of the button: a rule fixed after the fact should be able to reach the expenses
    /// that were filed before it was.
    ///
    /// Nothing is recorded in `seen_deliveries`: the line was delivered once already, and a
    /// second record of it is what would make the next statement upload skip a real payment.
    pub async fn resort(context: &Context, id: Uuid) -> FieldResult<GraphQLExpense> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let upstream_id = load_rendered(conn, user_id, id).await?.upstream_id;
        let now = Utc::now();

        let old_state = queries::load_expense(conn, upstream_id)
            .await?
            .ok_or_else(|| -> juniper::FieldError { "Diese Ausgabe gibt es nicht mehr.".into() })?;

        let delivery = rules::delivery_of(&old_state).ok_or_else(|| -> juniper::FieldError {
            "Diese Ausgabe hat keine Banktransaktion, nach der sie einsortiert werden könnte."
                .into()
        })?;

        let all_rules = scoped::automation_rules(user_id)
            .select(AutomationRule::as_select())
            .load(conn)
            .await?;

        let rule =
            rules::best_match(&all_rules, &delivery).ok_or_else(|| -> juniper::FieldError {
                "Keine Automatisierungsregel trifft auf diese Ausgabe zu.".into()
            })?;

        let template = queries::load_expense(conn, rule.template_id)
            .await?
            .ok_or_else(|| -> juniper::FieldError {
                format!("Die Regel {:?} hat keine Vorlage.", rule.title).into()
            })?;

        let mut sorted = rules::impose(&template, &delivery, now).map_err(|e| e.to_string())?;
        sorted.validate().map_err(|e| e.to_string())?;
        sorted.id = upstream_id;
        sorted.created_at = old_state.created_at;
        sorted.updated_at = now;

        conn.transaction::<_, diesel::result::Error, _>(async |conn| {
            queries::replace_expense(conn, &sorted).await?;

            diesel::insert_into(expense_changes::table)
                .values(&ExpenseChangelogEntry {
                    id: Uuid::now_v7(),
                    user_id: Some(user_id),
                    target_id: upstream_id,
                    date: now,
                    old_state: Some(Json(old_state)),
                    new_state: Some(Json(sorted.clone())),
                    tool: Some("Einsortieren".into()),
                    // a person pressed the button; the rule only decided the outcome
                    automatic: false,
                    confidence: 1.0,
                })
                .execute(conn)
                .await?;

            Ok(())
        })
        .await?;

        Ok(GraphQLExpense::new(
            load_rendered(conn, user_id, upstream_id).await?,
        ))
    }
}

type Conn = diesel_async::pooled_connection::deadpool::Object<AsyncPgConnection>;

async fn load_rendered(conn: &mut Conn, user_id: Uuid, id: Uuid) -> FieldResult<RenderedExpense> {
    scoped::expenses(user_id)
        .filter(expenses_rendered::id.eq(id))
        .select(RenderedExpense::as_select())
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| format!("Expense {id} not found").into())
}

async fn validate_and_unrender(
    conn: &mut Conn,
    user_id: Uuid,
    expense: RenderedExpenseInput,
) -> Result<Expense, Box<dyn Error>> {
    let account_ids: Vec<_> = expense.transactions.iter().map(|t| t.account_id).collect();
    let related_accounts = queries::query_associated_accounts(conn, &account_ids).await?;
    let categories = queries::query_categories(conn).await?;

    check_references(user_id, &expense, &related_accounts, &categories)?;

    let entry = Expense::from_rendered(expense);
    entry.validate()?;

    Ok(entry)
}

fn check_references(
    user_id: Uuid,
    expense: &RenderedExpenseInput,
    related_accounts: &[Account],
    categories: &[Category],
) -> Result<(), Box<dyn Error>> {
    // the foreign keys would catch unknown ids, but not that the expense has to touch this
    // user's money for them to be allowed to book it
    if expense
        .transactions
        .iter()
        .any(|t| !related_accounts.iter().any(|a| a.id == t.account_id))
    {
        return Err(format!(
            "user {user_id} tried to create an expense with unknown account id(s)"
        )
        .into());
    }

    if related_accounts.iter().all(|a| a.user_id != user_id) {
        return Err(format!(
            "user {user_id} tried to create an expense consisting only of accounts that don't belong to her"
        ).into());
    }

    if expense
        .categories
        .iter()
        .any(|c| !categories.iter().any(|k| k.id == c.category_id))
    {
        return Err(format!(
            "user {user_id} tried to create an expense with unknown category id(s)"
        )
        .into());
    }

    Ok(())
}
