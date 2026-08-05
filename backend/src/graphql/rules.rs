use uuid::Uuid;
use chrono::Utc;
use juniper::graphql_object;

use super::util;
use crate::graphql::root::{Context, require_user};
use crate::models::{
    AutomationRule, AutomationRuleInput, Expense, ExpenseDelivery, Uuid,
    PeriodicInterval, PeriodicRule, PeriodicRuleInput, SeenDelivery, rendering::RenderedExpense,
};
use crate::rules::DeliveryOutcome;

const AUTOMATION: &str = "automation_rules";
const PERIODIC: &str = "periodic_rules";

/// Callers address expenses by their *rendered* id, like everywhere else in this API, but a
/// rule has to reference the upstream expense because that is what gets copied. Looking the
/// rendered document up scoped by user resolves the id and authorizes in one step.
async fn resolve_template(
    context: &Context,
    user_id: Uuid,
    rendered_id: Uuid,
) -> juniper::FieldResult<Uuid> {
    let rendered = context
        .connection
        .collection::<RenderedExpense>("expenses_rendered")
        .find_one(doc! {"_id": *rendered_id, "userId": user_id})
        .await?
        .ok_or("template expense not found")?;

    Ok(rendered.upstream_id)
}

/// The reverse: the caller's own rendered view of the expense a rule copies.
async fn rendered_template_id(
    context: &Context,
    user_id: Uuid,
    upstream_id: Uuid,
) -> juniper::FieldResult<Option<Uuid>> {
    Ok(context
        .connection
        .collection::<RenderedExpense>("expenses_rendered")
        .find_one(doc! {"upstreamId": *upstream_id, "userId": user_id})
        .await?
        .map(|r| r.id))
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

    /// null when the template has no rendered document for this user, which means the
    /// template was deleted and the rule can no longer fire
    async fn template_id(
        &self,
        context: &Context,
    ) -> juniper::FieldResult<Option<Uuid>> {
        let user_id = require_user(context).await?;
        rendered_template_id(context, user_id, self.data.template_id).await
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
    async fn template_id(
        &self,
        context: &Context,
    ) -> juniper::FieldResult<Option<Uuid>> {
        let user_id = require_user(context).await?;
        rendered_template_id(context, user_id, self.data.template_id).await
    }

    /// the instants this rule would generate expenses for on the next run
    async fn pending(&self, context: &Context) -> juniper::FieldResult<Vec<chrono::DateTime<Utc>>> {
        require_user(context).await?;

        let Some(template) = context
            .connection
            .collection::<Expense>("expenses")
            .find_one(doc! {"_id": *self.data.template_id})
            .await?
        else {
            return Ok(vec![]);
        };

        Ok(crate::rules::occurrences(
            &self.data,
            &template,
            Utc::now() + crate::rules::PERIODIC_ADVANCE,
        ))
    }
}

pub struct AutomationRuleQuery;

#[graphql_object(Context = Context)]
impl AutomationRuleQuery {
    pub async fn list(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
        filter: Option<String>,
        sort: Option<String>,
    ) -> juniper::FieldResult<Vec<GraphQLAutomationRule>> {
        let user_id = require_user(context).await?;

        Ok(util::list_query::<AutomationRule>(
            AUTOMATION,
            &context.connection,
            limit,
            skip,
            filter,
            sort,
            Some(doc!("userId": user_id)),
        )
        .await?
        .1
        .into_iter()
        .map(|data| GraphQLAutomationRule { data })
        .collect())
    }

    pub async fn get(
        context: &Context,
        id: Uuid,
    ) -> juniper::FieldResult<GraphQLAutomationRule> {
        let user_id = require_user(context).await?;

        let data = context
            .connection
            .collection::<AutomationRule>(AUTOMATION)
            .find_one(doc! {"_id": *id, "userId": user_id})
            .await?
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
    ) -> juniper::FieldResult<GraphQLAutomationRule> {
        let user_id = require_user(context).await?;
        let template_id = resolve_template(context, user_id, rule.template_id).await?;

        let mut data = rule.upgrade(Uuid::now_v7().into(), user_id.into(), None, Utc::now());
        data.template_id = template_id;
        context
            .connection
            .collection::<AutomationRule>(AUTOMATION)
            .insert_one(&data)
            .await?;

        Ok(GraphQLAutomationRule { data })
    }

    pub async fn update(
        context: &Context,
        id: Uuid,
        rule: AutomationRuleInput,
    ) -> juniper::FieldResult<GraphQLAutomationRule> {
        let user_id = require_user(context).await?;
        let template_id = resolve_template(context, user_id, rule.template_id).await?;

        let collection = context.connection.collection::<AutomationRule>(AUTOMATION);
        let existing = collection
            .find_one(doc! {"_id": *id, "userId": user_id})
            .await?
            .ok_or("automation rule not found")?;

        // last_delivery is bookkeeping the rule engine owns, not something callers set
        let mut data = rule.upgrade(id, user_id.into(), existing.last_delivery, Utc::now());
        data.template_id = template_id;
        collection
            .replace_one(doc! {"_id": *id, "userId": user_id}, &data)
            .await?;

        Ok(GraphQLAutomationRule { data })
    }

    pub async fn delete(context: &Context, id: Uuid) -> juniper::FieldResult<bool> {
        let user_id = require_user(context).await?;

        let count = context
            .connection
            .collection::<AutomationRule>(AUTOMATION)
            .delete_one(doc! {"_id": *id, "userId": user_id})
            .await?
            .deleted_count;

        Ok(count > 0)
    }
}

pub struct PeriodicRuleQuery;

#[graphql_object(Context = Context)]
impl PeriodicRuleQuery {
    pub async fn list(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
        filter: Option<String>,
        sort: Option<String>,
    ) -> juniper::FieldResult<Vec<GraphQLPeriodicRule>> {
        let user_id = require_user(context).await?;

        Ok(util::list_query::<PeriodicRule>(
            PERIODIC,
            &context.connection,
            limit,
            skip,
            filter,
            sort,
            Some(doc!("userId": user_id)),
        )
        .await?
        .1
        .into_iter()
        .map(|data| GraphQLPeriodicRule { data })
        .collect())
    }

    pub async fn get(
        context: &Context,
        id: Uuid,
    ) -> juniper::FieldResult<GraphQLPeriodicRule> {
        let user_id = require_user(context).await?;

        let data = context
            .connection
            .collection::<PeriodicRule>(PERIODIC)
            .find_one(doc! {"_id": *id, "userId": user_id})
            .await?
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
    ) -> juniper::FieldResult<GraphQLPeriodicRule> {
        let user_id = require_user(context).await?;
        let template_id = resolve_template(context, user_id, rule.template_id).await?;

        let mut data = rule.upgrade(Uuid::now_v7().into(), user_id.into(), Utc::now());
        data.template_id = template_id;
        context
            .connection
            .collection::<PeriodicRule>(PERIODIC)
            .insert_one(&data)
            .await?;

        Ok(GraphQLPeriodicRule { data })
    }

    pub async fn update(
        context: &Context,
        id: Uuid,
        rule: PeriodicRuleInput,
    ) -> juniper::FieldResult<GraphQLPeriodicRule> {
        let user_id = require_user(context).await?;
        let template_id = resolve_template(context, user_id, rule.template_id).await?;

        let collection = context.connection.collection::<PeriodicRule>(PERIODIC);
        collection
            .find_one(doc! {"_id": *id, "userId": user_id})
            .await?
            .ok_or("periodic rule not found")?;

        let mut data = rule.upgrade(id, user_id.into(), Utc::now());
        data.template_id = template_id;
        collection
            .replace_one(doc! {"_id": *id, "userId": user_id}, &data)
            .await?;

        Ok(GraphQLPeriodicRule { data })
    }

    pub async fn delete(context: &Context, id: Uuid) -> juniper::FieldResult<bool> {
        let user_id = require_user(context).await?;

        let count = context
            .connection
            .collection::<PeriodicRule>(PERIODIC)
            .delete_one(doc! {"_id": *id, "userId": user_id})
            .await?
            .deleted_count;

        Ok(count > 0)
    }

    /// Generates every expense that is currently due, across all of this user's rules.
    pub async fn run(context: &Context) -> juniper::FieldResult<i32> {
        require_user(context).await?;

        let count = crate::rules::generate_periodic_expenses(&context.connection, Utc::now())
            .await
            .map_err(|e| e.to_string())?;

        Ok(count as i32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, juniper::GraphQLEnum)]
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

    /// the created expense, as this caller's rendered id
    async fn expense_id(&self, context: &Context) -> juniper::FieldResult<Option<Uuid>> {
        let Some(upstream) = self.expense_id else {
            return Ok(None);
        };

        let user_id = require_user(context).await?;
        rendered_template_id(context, user_id, upstream).await
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
    ) -> juniper::FieldResult<Vec<DeliveryResult>> {
        let user_id = require_user(context).await?;

        // an account the caller does not own would let them probe other people's rules
        for delivery in &deliveries {
            let account = context
                .account_loader
                .try_load(delivery.account_id)
                .await
                .map_err(|_| "unknown account")?;

            if *account.user_id != user_id {
                return Err("cannot deliver to an account that is not yours".into());
            }
        }

        let outcomes =
            crate::rules::deliver_expenses(&context.connection, user_id, &deliveries, Utc::now())
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

    /// the expense this delivery produced, as this caller's rendered id
    async fn expense_id(&self, context: &Context) -> juniper::FieldResult<Option<Uuid>> {
        let Some(upstream) = self.data.expense_id else {
            return Ok(None);
        };

        let user_id = require_user(context).await?;
        rendered_template_id(context, user_id, upstream).await
    }
}

pub struct DeliveryQuery;

#[graphql_object(Context = Context)]
impl DeliveryQuery {
    pub async fn list(
        context: &Context,
        limit: Option<i32>,
        skip: Option<i32>,
        filter: Option<String>,
        sort: Option<String>,
    ) -> juniper::FieldResult<Vec<GraphQLSeenDelivery>> {
        let user_id = require_user(context).await?;

        Ok(util::list_query::<SeenDelivery>(
            "seen_deliveries",
            &context.connection,
            limit,
            skip,
            filter,
            sort,
            Some(doc!("userId": user_id)),
        )
        .await?
        .1
        .into_iter()
        .map(|data| GraphQLSeenDelivery { data })
        .collect())
    }
}
