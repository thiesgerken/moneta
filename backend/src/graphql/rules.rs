use uuid::Uuid;

use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use juniper::{FieldResult, GraphQLEnum, graphql_object};

use super::util::{self, SortDirection};
use crate::graphql::root::{Context, require_user};
use crate::models::{
    AutomationRule, AutomationRuleInput, ExpenseDelivery, PeriodicInterval, PeriodicRule,
    PeriodicRuleInput, SeenDelivery,
};
use crate::order_by;
use crate::queries;
use crate::rules::DeliveryOutcome;
use crate::schema::{automation_rules, periodic_rules, seen_deliveries};
use crate::schema_views::expenses_rendered;

type Conn = diesel_async::pooled_connection::deadpool::Object<diesel_async::AsyncPgConnection>;

/// Whether this user can see the expense a rule points at.
///
/// Callers address expenses by their rendered id, which is the upstream id -- a reader only
/// ever sees their own rendering. What still has to be checked is that they see it at all,
/// since a rule pointing at somebody else's expense would copy money they cannot read.
async fn visible_expense(conn: &mut Conn, user_id: Uuid, id: Uuid) -> FieldResult<Option<Uuid>> {
    Ok(expenses_rendered::table
        .filter(expenses_rendered::id.eq(id))
        .filter(expenses_rendered::user_id.eq(user_id))
        .select(expenses_rendered::upstream_id)
        .first(conn)
        .await
        .optional()?)
}

async fn require_visible_expense(conn: &mut Conn, user_id: Uuid, id: Uuid) -> FieldResult<Uuid> {
    visible_expense(conn, user_id, id)
        .await?
        .ok_or_else(|| "template expense not found".into())
}

/// Both rule types are wrapped rather than derived so `templateId` can be handed back as the
/// caller's own rendered expense id — the same id they passed in, and the only one the rest
/// of the API accepts.
#[derive(Debug, Clone)]
pub struct GraphQLAutomationRule {
    data: AutomationRule,
}

#[graphql_object(Context = Context, name = "AutomationRule")]
impl GraphQLAutomationRule {
    fn id(&self) -> Uuid {
        self.data.id
    }
    fn title(&self) -> &str {
        &self.data.title
    }
    fn priority(&self) -> i32 {
        self.data.priority
    }
    fn regex_time(&self) -> Option<&str> {
        self.data.regex_time.as_deref()
    }
    fn regex_statement(&self) -> Option<&str> {
        self.data.regex_statement.as_deref()
    }
    fn filter_amount(&self) -> Option<i32> {
        self.data.filter_amount
    }
    fn filter_account_id(&self) -> Option<Uuid> {
        self.data.filter_account_id
    }
    fn last_delivery(&self) -> Option<chrono::DateTime<Utc>> {
        self.data.last_delivery
    }
    fn created_at(&self) -> chrono::DateTime<Utc> {
        self.data.created_at
    }
    fn updated_at(&self) -> chrono::DateTime<Utc> {
        self.data.updated_at
    }

    /// null when the caller can no longer see the template, which means the rule cannot fire
    /// for them
    async fn template_id(&self, context: &Context) -> FieldResult<Option<Uuid>> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;
        visible_expense(conn, user_id, self.data.template_id).await
    }
}

#[derive(Debug, Clone)]
pub struct GraphQLPeriodicRule {
    data: PeriodicRule,
}

#[graphql_object(Context = Context, name = "PeriodicRule")]
impl GraphQLPeriodicRule {
    fn id(&self) -> Uuid {
        self.data.id
    }
    fn title(&self) -> &str {
        &self.data.title
    }
    fn period(&self) -> PeriodicInterval {
        self.data.period
    }
    fn title_replacement(&self) -> Option<&str> {
        self.data.title_replacement.as_deref()
    }
    fn amount_replacement(&self) -> Option<i32> {
        self.data.amount_replacement
    }
    fn last_creation(&self) -> chrono::DateTime<Utc> {
        self.data.last_creation
    }
    fn created_at(&self) -> chrono::DateTime<Utc> {
        self.data.created_at
    }
    fn updated_at(&self) -> chrono::DateTime<Utc> {
        self.data.updated_at
    }

    /// see `AutomationRule.templateId`
    async fn template_id(&self, context: &Context) -> FieldResult<Option<Uuid>> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;
        visible_expense(conn, user_id, self.data.template_id).await
    }

    /// the instants this rule would generate expenses for on the next run
    async fn pending(&self, context: &Context) -> FieldResult<Vec<chrono::DateTime<Utc>>> {
        require_user(context).await?;
        let conn = &mut context.conn().await?;

        let Some(template) = queries::load_expense(conn, self.data.template_id).await? else {
            return Ok(vec![]);
        };

        Ok(crate::rules::occurrences(
            &self.data,
            &template,
            Utc::now() + crate::rules::PERIODIC_ADVANCE,
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, GraphQLEnum)]
pub enum AutomationRuleSortField {
    #[default]
    Priority,
    Title,
    LastDelivery,
}

pub struct AutomationRuleQuery;

#[graphql_object(Context = Context)]
impl AutomationRuleQuery {
    pub async fn list(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
        search: Option<String>,
        sort_by: Option<AutomationRuleSortField>,
        sort_dir: Option<SortDirection>,
    ) -> FieldResult<Vec<GraphQLAutomationRule>> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let (limit, offset) = util::page(limit, skip);
        let direction = sort_dir.unwrap_or_default();

        let mut query = automation_rules::table
            .into_boxed()
            .filter(automation_rules::user_id.eq(user_id));

        if let Some(term) = search.filter(|s| !s.trim().is_empty()) {
            let pattern = util::ilike_pattern(term.trim());
            query = query.filter(
                automation_rules::title
                    .ilike(pattern.clone())
                    .or(automation_rules::regex_statement.ilike(pattern)),
            );
        }

        let query = match sort_by.unwrap_or_default() {
            AutomationRuleSortField::Priority => {
                order_by!(query, direction, automation_rules::priority)
            }
            AutomationRuleSortField::Title => order_by!(query, direction, automation_rules::title),
            AutomationRuleSortField::LastDelivery => {
                order_by!(query, direction, automation_rules::last_delivery)
            }
        };

        Ok(query
            .then_order_by(automation_rules::id.asc())
            .limit(limit)
            .offset(offset)
            .select(AutomationRule::as_select())
            .load(conn)
            .await?
            .into_iter()
            .map(|data| GraphQLAutomationRule { data })
            .collect())
    }

    pub async fn get(context: &Context, id: Uuid) -> FieldResult<GraphQLAutomationRule> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let data = automation_rules::table
            .find(id)
            .filter(automation_rules::user_id.eq(user_id))
            .select(AutomationRule::as_select())
            .first(conn)
            .await
            .optional()?
            .ok_or("automation rule not found")?;

        Ok(GraphQLAutomationRule { data })
    }
}

pub struct AutomationRuleMutation;

#[graphql_object(Context = Context)]
impl AutomationRuleMutation {
    pub async fn create(
        context: &Context,
        rule: AutomationRuleInput,
    ) -> FieldResult<GraphQLAutomationRule> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let template_id = require_visible_expense(conn, user_id, rule.template_id).await?;

        let mut data = rule.upgrade(Uuid::now_v7(), user_id, None, Utc::now());
        data.template_id = template_id;

        diesel::insert_into(automation_rules::table)
            .values(&data)
            .execute(conn)
            .await?;

        Ok(GraphQLAutomationRule { data })
    }

    pub async fn update(
        context: &Context,
        id: Uuid,
        rule: AutomationRuleInput,
    ) -> FieldResult<GraphQLAutomationRule> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let template_id = require_visible_expense(conn, user_id, rule.template_id).await?;

        let existing = automation_rules::table
            .find(id)
            .filter(automation_rules::user_id.eq(user_id))
            .select(AutomationRule::as_select())
            .first(conn)
            .await
            .optional()?
            .ok_or("automation rule not found")?;

        // last_delivery is bookkeeping the rule engine owns, not something callers set
        let mut data = rule.upgrade(id, user_id, existing.last_delivery, Utc::now());
        data.template_id = template_id;
        data.created_at = existing.created_at;

        diesel::update(automation_rules::table.find(id))
            .set(&data)
            .execute(conn)
            .await?;

        Ok(GraphQLAutomationRule { data })
    }

    pub async fn delete(context: &Context, id: Uuid) -> FieldResult<bool> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let count = diesel::delete(
            automation_rules::table
                .find(id)
                .filter(automation_rules::user_id.eq(user_id)),
        )
        .execute(conn)
        .await?;

        Ok(count > 0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, GraphQLEnum)]
pub enum PeriodicRuleSortField {
    #[default]
    Title,
    Period,
    LastCreation,
}

pub struct PeriodicRuleQuery;

#[graphql_object(Context = Context)]
impl PeriodicRuleQuery {
    pub async fn list(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
        search: Option<String>,
        sort_by: Option<PeriodicRuleSortField>,
        sort_dir: Option<SortDirection>,
    ) -> FieldResult<Vec<GraphQLPeriodicRule>> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let (limit, offset) = util::page(limit, skip);
        let direction = sort_dir.unwrap_or_default();

        let mut query = periodic_rules::table
            .into_boxed()
            .filter(periodic_rules::user_id.eq(user_id));

        if let Some(term) = search.filter(|s| !s.trim().is_empty()) {
            query = query.filter(periodic_rules::title.ilike(util::ilike_pattern(term.trim())));
        }

        let query = match sort_by.unwrap_or_default() {
            PeriodicRuleSortField::Title => order_by!(query, direction, periodic_rules::title),
            PeriodicRuleSortField::Period => order_by!(query, direction, periodic_rules::period),
            PeriodicRuleSortField::LastCreation => {
                order_by!(query, direction, periodic_rules::last_creation)
            }
        };

        Ok(query
            .then_order_by(periodic_rules::id.asc())
            .limit(limit)
            .offset(offset)
            .select(PeriodicRule::as_select())
            .load(conn)
            .await?
            .into_iter()
            .map(|data| GraphQLPeriodicRule { data })
            .collect())
    }

    pub async fn get(context: &Context, id: Uuid) -> FieldResult<GraphQLPeriodicRule> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let data = periodic_rules::table
            .find(id)
            .filter(periodic_rules::user_id.eq(user_id))
            .select(PeriodicRule::as_select())
            .first(conn)
            .await
            .optional()?
            .ok_or("periodic rule not found")?;

        Ok(GraphQLPeriodicRule { data })
    }
}

pub struct PeriodicRuleMutation;

#[graphql_object(Context = Context)]
impl PeriodicRuleMutation {
    pub async fn create(
        context: &Context,
        rule: PeriodicRuleInput,
    ) -> FieldResult<GraphQLPeriodicRule> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let template_id = require_visible_expense(conn, user_id, rule.template_id).await?;

        let mut data = rule.upgrade(Uuid::now_v7(), user_id, Utc::now());
        data.template_id = template_id;

        diesel::insert_into(periodic_rules::table)
            .values(&data)
            .execute(conn)
            .await?;

        Ok(GraphQLPeriodicRule { data })
    }

    pub async fn update(
        context: &Context,
        id: Uuid,
        rule: PeriodicRuleInput,
    ) -> FieldResult<GraphQLPeriodicRule> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let template_id = require_visible_expense(conn, user_id, rule.template_id).await?;

        let existing = periodic_rules::table
            .find(id)
            .filter(periodic_rules::user_id.eq(user_id))
            .select(PeriodicRule::as_select())
            .first(conn)
            .await
            .optional()?
            .ok_or("periodic rule not found")?;

        let mut data = rule.upgrade(id, user_id, Utc::now());
        data.template_id = template_id;
        data.created_at = existing.created_at;

        diesel::update(periodic_rules::table.find(id))
            .set(&data)
            .execute(conn)
            .await?;

        Ok(GraphQLPeriodicRule { data })
    }

    pub async fn delete(context: &Context, id: Uuid) -> FieldResult<bool> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let count = diesel::delete(
            periodic_rules::table
                .find(id)
                .filter(periodic_rules::user_id.eq(user_id)),
        )
        .execute(conn)
        .await?;

        Ok(count > 0)
    }

    /// Generates every expense that is currently due from this user's own rules. The scheduled
    /// job covers everybody; this is the button for not waiting for it.
    pub async fn run(context: &Context) -> FieldResult<i32> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let count = crate::rules::generate_periodic_expenses(conn, Some(user_id), Utc::now())
            .await
            .map_err(|e| e.to_string())?;

        Ok(count as i32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, GraphQLEnum)]
pub enum DeliveryStatus {
    /// an expense was created
    Created,
    /// already delivered before, so nothing was done
    Duplicate,
    /// no automation rule matched; the line is remembered but produced no expense
    Unmatched,
    /// a rule matched but could not be applied, and the line was *not* remembered so that
    /// fixing the rule and re-uploading picks it up
    Failed,
}

pub struct DeliveryResult {
    status: DeliveryStatus,
    expense_id: Option<Uuid>,
    rule_id: Option<Uuid>,
    message: Option<String>,
}

#[graphql_object(Context = Context, name = "ExpenseDeliveryResult")]
impl DeliveryResult {
    fn status(&self) -> DeliveryStatus {
        self.status
    }

    fn rule_id(&self) -> Option<Uuid> {
        self.rule_id
    }

    fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// the created expense, or null if the caller cannot see it
    async fn expense_id(&self, context: &Context) -> FieldResult<Option<Uuid>> {
        let Some(upstream) = self.expense_id else {
            return Ok(None);
        };

        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;
        visible_expense(conn, user_id, upstream).await
    }
}

pub struct DeliveryMutation;

#[graphql_object(Context = Context)]
impl DeliveryMutation {
    /// Hands bank statement lines to the automation rules.
    ///
    /// Safe to call with overlapping batches: a line already delivered is reported as a
    /// duplicate rather than creating a second expense.
    pub async fn submit(
        context: &Context,
        deliveries: Vec<ExpenseDelivery>,
    ) -> FieldResult<Vec<DeliveryResult>> {
        let user_id = require_user(context).await?;

        // an account the caller does not own would let them probe other people's rules
        for delivery in &deliveries {
            let account = context
                .account_loader
                .try_load(delivery.account_id)
                .await
                .map_err(|_| "unknown account")?;

            if account.user_id != user_id {
                return Err("cannot deliver to an account that is not yours".into());
            }
        }

        let conn = &mut context.conn().await?;
        let outcomes = crate::rules::deliver_expenses(conn, user_id, &deliveries, Utc::now())
            .await
            .map_err(|e| e.to_string())?;

        Ok(outcomes
            .into_iter()
            .map(|o| match o {
                DeliveryOutcome::Created {
                    expense_id,
                    rule_id,
                } => DeliveryResult {
                    status: DeliveryStatus::Created,
                    expense_id: Some(expense_id),
                    rule_id: Some(rule_id),
                    message: None,
                },
                DeliveryOutcome::Duplicate => DeliveryResult {
                    status: DeliveryStatus::Duplicate,
                    expense_id: None,
                    rule_id: None,
                    message: None,
                },
                DeliveryOutcome::Unmatched => DeliveryResult {
                    status: DeliveryStatus::Unmatched,
                    expense_id: None,
                    rule_id: None,
                    message: None,
                },
                DeliveryOutcome::Failed { message } => DeliveryResult {
                    status: DeliveryStatus::Failed,
                    expense_id: None,
                    rule_id: None,
                    message: Some(message),
                },
            })
            .collect())
    }
}

/// A delivery that has been handed in, and what became of it.
pub struct GraphQLSeenDelivery {
    data: SeenDelivery,
}

#[graphql_object(Context = Context, name = "SeenDelivery")]
impl GraphQLSeenDelivery {
    fn id(&self) -> Uuid {
        self.data.id
    }
    fn account_id(&self) -> Uuid {
        self.data.account_id
    }
    fn date(&self) -> chrono::DateTime<Utc> {
        self.data.date
    }
    fn amount(&self) -> i32 {
        self.data.amount
    }
    fn statement(&self) -> &str {
        &self.data.statement
    }
    fn created_at(&self) -> chrono::DateTime<Utc> {
        self.data.created_at
    }

    /// false when no rule matched, which is the signal that a rule is missing
    fn matched(&self) -> bool {
        self.data.expense_id.is_some()
    }

    /// the expense this delivery produced, or null if the caller cannot see it
    async fn expense_id(&self, context: &Context) -> FieldResult<Option<Uuid>> {
        let Some(upstream) = self.data.expense_id else {
            return Ok(None);
        };

        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;
        visible_expense(conn, user_id, upstream).await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, GraphQLEnum)]
pub enum DeliverySortField {
    #[default]
    Date,
    Amount,
}

pub struct DeliveryQuery;

#[graphql_object(Context = Context)]
impl DeliveryQuery {
    pub async fn list(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
        search: Option<String>,
        only_unmatched: Option<bool>,
        sort_by: Option<DeliverySortField>,
        sort_dir: Option<SortDirection>,
    ) -> FieldResult<Vec<GraphQLSeenDelivery>> {
        let user_id = require_user(context).await?;
        let conn = &mut context.conn().await?;

        let (limit, offset) = util::page(limit, skip);
        let direction = sort_dir.unwrap_or_default();

        let mut query = seen_deliveries::table
            .into_boxed()
            .filter(seen_deliveries::user_id.eq(user_id));

        if let Some(term) = search.filter(|s| !s.trim().is_empty()) {
            query =
                query.filter(seen_deliveries::statement.ilike(util::ilike_pattern(term.trim())));
        }
        if only_unmatched.unwrap_or(false) {
            query = query.filter(seen_deliveries::expense_id.is_null());
        }

        let query = match sort_by.unwrap_or_default() {
            DeliverySortField::Date => order_by!(query, direction, seen_deliveries::date),
            DeliverySortField::Amount => order_by!(query, direction, seen_deliveries::amount),
        };

        Ok(query
            .then_order_by(seen_deliveries::id.asc())
            .limit(limit)
            .offset(offset)
            .select(SeenDelivery::as_select())
            .load(conn)
            .await?
            .into_iter()
            .map(|data| GraphQLSeenDelivery { data })
            .collect())
    }
}
