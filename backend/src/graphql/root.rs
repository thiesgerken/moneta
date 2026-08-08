use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use uuid::Uuid;

use juniper::graphql_object;
use log::info;
use tokio::sync::RwLock;

use crate::graphql::{
    accounts::{AccountMutation, AccountQuery},
    analysis::AnalysisQuery,
    balances::{BalanceMutation, BalanceQuery},
    categories::{CategoryMutation, CategoryQuery},
    expenses::{ExpenseMutation, ExpenseQuery},
    rules::{
        AutomationRuleMutation, AutomationRuleQuery, DeliveryMutation, DeliveryQuery,
        PeriodicRuleMutation, PeriodicRuleQuery,
    },
    users::{UserMutation, UserQuery},
};

use super::{
    accounts::{AccountLoader, get_account_loader},
    cache::AnalysisCache,
    categories::{CategoryLoader, get_category_loader},
    users::{UserLoader, get_user_loader},
};

pub struct Context {
    pub pool: crate::Pool,
    pub config: crate::config::Config,
    pub user_id: Arc<RwLock<Option<Uuid>>>,
    pub cache: Arc<AnalysisCache>,
    /// Set by the mutation root, read once the request is done. The cache is dropped after
    /// the write has committed rather than when the field is entered: clearing first leaves a
    /// concurrent read free to refill it with what the write is in the middle of replacing.
    pub wrote: AtomicBool,

    pub account_loader: AccountLoader,
    pub category_loader: CategoryLoader,
    pub user_loader: UserLoader,
}

impl juniper::Context for Context {}
pub struct Query;

pub struct Mutation;

pub type Schema = juniper::RootNode<Query, Mutation, juniper::EmptySubscription<Context>>;

impl Context {
    /// Checking out a connection can fail (the pool is bounded), so resolvers go through
    /// this rather than unwrapping in a dozen places.
    pub async fn conn(
        &self,
    ) -> juniper::FieldResult<
        diesel_async::pooled_connection::deadpool::Object<diesel_async::AsyncPgConnection>,
    > {
        Ok(self.pool.get().await?)
    }
}

pub async fn require_user(context: &Context) -> juniper::FieldResult<Uuid> {
    context.user_id.read().await.ok_or_else(|| {
        // The frontend sends people to the login form on this code and on nothing else, so a
        // query that merely failed is not mistaken for a session that ended.
        juniper::FieldError::new(
            "User is not logged in",
            juniper::graphql_value!({ "code": "UNAUTHENTICATED" }),
        )
    })
}

pub async fn graphql_handler(
    pool: &rocket::State<crate::Pool>,
    config: &rocket::State<crate::config::Config>,
    cache: &rocket::State<Arc<AnalysisCache>>,
    cookies: &rocket::http::CookieJar<'_>,
    request: juniper_rocket::GraphQLRequest,
    schema: &rocket::State<Schema>,
) -> juniper_rocket::GraphQLResponse {
    let user_id = cookies
        .get_private("user_id")
        .and_then(|c| c.value().parse().ok());
    let user_id_writer = Arc::new(RwLock::new(user_id));

    let context = Context {
        config: (*config).clone(),
        pool: (*pool).clone(),
        user_id: user_id_writer.clone(),
        cache: (*cache).clone(),
        wrote: AtomicBool::new(false),
        account_loader: get_account_loader((*pool).clone()),
        category_loader: get_category_loader((*pool).clone()),
        user_loader: get_user_loader((*pool).clone()),
    };

    let r = request.execute(schema, &context).await;

    if context.wrote.load(Ordering::Relaxed) {
        context.cache.clear();
    }

    let new_user_id = user_id_writer.read().await;
    if *new_user_id != user_id {
        if let Some(uid) = *new_user_id {
            info!("Login for user_id {}", uid);
            cookies.add_private(rocket::http::Cookie::new("user_id", uid.to_string()));
        } else {
            info!("Logout for user_id {}", user_id.unwrap());
            cookies.remove_private(rocket::http::Cookie::build("user_id"));
        }
    }

    r
}

#[graphql_object(
    Context = Context,
)]
impl Query {
    fn api_version() -> &'static str {
        "1.0"
    }

    fn users() -> UserQuery {
        UserQuery
    }

    fn accounts() -> AccountQuery {
        AccountQuery
    }

    fn balances() -> BalanceQuery {
        BalanceQuery
    }

    fn expenses() -> ExpenseQuery {
        ExpenseQuery
    }

    fn categories() -> CategoryQuery {
        CategoryQuery
    }

    fn analysis() -> AnalysisQuery {
        AnalysisQuery
    }

    fn automation_rules() -> AutomationRuleQuery {
        AutomationRuleQuery
    }

    fn periodic_rules() -> PeriodicRuleQuery {
        PeriodicRuleQuery
    }

    fn deliveries() -> DeliveryQuery {
        DeliveryQuery
    }
}

#[graphql_object(
    Context = Context,
)]
impl Mutation {
    // A `///` here would land in the SDL. The one write that does not drop the analysis
    // cache: this branch is login, logout and the visit marker, and the marker is recorded on
    // every single dashboard load. Dropping it here would mean the dashboard invalidates the
    // answers it is about to ask for.
    fn users() -> UserMutation {
        UserMutation
    }

    fn accounts(context: &Context) -> AccountMutation {
        context.wrote.store(true, Ordering::Relaxed);
        AccountMutation
    }

    fn balances(context: &Context) -> BalanceMutation {
        context.wrote.store(true, Ordering::Relaxed);
        BalanceMutation
    }

    fn expenses(context: &Context) -> ExpenseMutation {
        context.wrote.store(true, Ordering::Relaxed);
        ExpenseMutation
    }

    fn categories(context: &Context) -> CategoryMutation {
        context.wrote.store(true, Ordering::Relaxed);
        CategoryMutation
    }

    fn automation_rules(context: &Context) -> AutomationRuleMutation {
        context.wrote.store(true, Ordering::Relaxed);
        AutomationRuleMutation
    }

    fn periodic_rules(context: &Context) -> PeriodicRuleMutation {
        context.wrote.store(true, Ordering::Relaxed);
        PeriodicRuleMutation
    }

    fn deliveries(context: &Context) -> DeliveryMutation {
        context.wrote.store(true, Ordering::Relaxed);
        DeliveryMutation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Which mutations drop the analysis cache, checked against the schema rather than by
    /// reading the code: the exception for `users` is easy to forget and expensive to get
    /// wrong in either direction — forget it and the dashboard invalidates its own answers on
    /// every load, drop it and a write leaves stale months on the screen for ten minutes.
    async fn wrote(document: &str) -> bool {
        // never connected to: both branches below stop at `require_user`, and deadpool only
        // dials when a connection is actually taken
        let pool = crate::pool("postgres://nobody@127.0.0.1:1/nothing", "test").unwrap();
        let context = Context {
            config: crate::config::Config::default(),
            pool: pool.clone(),
            user_id: Arc::new(RwLock::new(None)),
            cache: Arc::new(AnalysisCache::default()),
            wrote: AtomicBool::new(false),
            account_loader: get_account_loader(pool.clone()),
            category_loader: get_category_loader(pool.clone()),
            user_loader: get_user_loader(pool),
        };

        let schema = Schema::new(Query, Mutation, juniper::EmptySubscription::new());
        let _ = juniper::execute(
            document,
            None,
            &schema,
            &juniper::Variables::new(),
            &context,
        )
        .await;

        context.wrote.load(Ordering::Relaxed)
    }

    #[tokio::test]
    async fn a_write_invalidates_the_analysis_cache() {
        assert!(
            wrote("mutation { expenses { delete(id: \"00000000-0000-0000-0000-000000000000\") } }")
                .await
        );
    }

    #[tokio::test]
    async fn recording_a_visit_does_not() {
        assert!(!wrote("mutation { users { visit { previous } } }").await);
    }
}
