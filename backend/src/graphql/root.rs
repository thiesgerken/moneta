use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use uuid::Uuid;

use juniper::graphql_object;
use log::{debug, info, warn};
use tokio::sync::RwLock;

use crate::graphql::{
    accounts::{AccountMutation, AccountQuery},
    analysis::AnalysisQuery,
    api_keys::{ApiKeyMutation, ApiKeyQuery},
    balances::{BalanceMutation, BalanceQuery},
    categories::{CategoryMutation, CategoryQuery},
    expenses::{ExpenseMutation, ExpenseQuery},
    investments::InvestmentQuery,
    rules::{
        AutomationRuleMutation, AutomationRuleQuery, DeliveryMutation, DeliveryQuery,
        PeriodicRuleMutation, PeriodicRuleQuery,
    },
    users::{UserMutation, UserQuery},
};

use super::{
    accounts::{AccountLoader, AccountUsageLoader, get_account_loader, get_account_usage_loader},
    cache::AnalysisCache,
    categories::{
        CategoryLoader, CategoryUsageLoader, get_category_loader, get_category_usage_loader,
    },
    users::{UserLoader, get_user_loader},
};

pub struct Context {
    pub pool: crate::Pool,
    pub config: crate::config::Config,
    pub user_id: Arc<RwLock<Option<Uuid>>>,
    pub credential: Credential,
    pub cache: Arc<AnalysisCache>,
    /// Set by the mutation root, read once the request is done. The cache is dropped after
    /// the write has committed rather than when the field is entered: clearing first leaves a
    /// concurrent read free to refill it with what the write is in the middle of replacing.
    pub wrote: AtomicBool,

    pub account_loader: AccountLoader,
    pub category_loader: CategoryLoader,
    /// How often each account and category has been used lately, which is what the pickers
    /// sort on. Separate loaders rather than a field on the row: it is a count over other
    /// tables, and only the two dialogs that offer a choice ever ask for it.
    pub account_usage_loader: AccountUsageLoader,
    pub category_usage_loader: CategoryUsageLoader,
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Credential {
    /// The session cookie, or nobody at all.
    Session,
    /// `Remote-User` from a trusted proxy. A browser like a session, but one the proxy keeps:
    /// it neither writes nor clears the cookie.
    TrustedHeader,
    ApiKey(crate::api_keys::ApiKeyAccess),
}

/// What a request offers as its credential. Only one of them counts, in this order: an explicit
/// Authorization header, even a malformed one, then `Remote-User` from a trusted proxy, then the
/// session cookie. The proxy before the cookie: it says who is at the browser on every request,
/// and a cookie left behind by a password login must not outvote it.
pub enum Presented<'r> {
    /// Empty when the header came more than once, which no token matches.
    Authorization(&'r str),
    RemoteUser(&'r str),
    /// More than one `Remote-User`. A proxy that appends rather than replaces puts the one the
    /// client sent first, so none of them can be believed.
    AmbiguousRemoteUser,
    Cookie(Option<Uuid>),
}

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for Presented<'r> {
    type Error = std::convert::Infallible;

    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let presented = if let Some(header) = single(request, "Authorization") {
            Self::Authorization(header.unwrap_or(""))
        } else if let Some(name) = single(request, "Remote-User").filter(|_| from_proxy(request)) {
            name.map_or(Self::AmbiguousRemoteUser, Self::RemoteUser)
        } else {
            Self::Cookie(
                request
                    .cookies()
                    .get_private("user_id")
                    .and_then(|c| c.value().parse().ok()),
            )
        };
        rocket::request::Outcome::Success(presented)
    }
}

/// `None` without the header, `Some(None)` when it came more than once.
fn single<'r>(request: &'r rocket::Request<'_>, name: &str) -> Option<Option<&'r str>> {
    let mut headers = request.headers().get(name);
    let first = headers.next()?;
    Some(if headers.next().is_some() {
        None
    } else {
        Some(first)
    })
}

/// Whether the request came straight from a proxy `Remote-User` is believed from.
fn from_proxy(request: &rocket::Request<'_>) -> bool {
    let api = &request
        .rocket()
        .state::<crate::config::Config>()
        .expect("the config is managed")
        .api;
    // The socket peer, not `client_ip()`: that one prefers rocket's `ip_header` (X-Real-IP by
    // default), which whoever sends the request gets to write.
    let peer = request.remote().map(|address| address.ip());

    if !api.trusted_header_authentication {
        debug!("Ignoring Remote-User from {peer:?}: header authentication is off");
        false
    } else if peer.is_some_and(|peer| api.trusts(peer)) {
        true
    } else {
        warn!("Ignoring Remote-User from {peer:?}, which is not a trusted proxy");
        false
    }
}

fn unauthenticated() -> juniper::FieldError {
    juniper::FieldError::new(
        "Missing or invalid credential",
        juniper::graphql_value!({"code": "UNAUTHENTICATED"}),
    )
}

fn forbidden() -> juniper::FieldError {
    juniper::FieldError::new(
        "This credential does not permit this operation",
        juniper::graphql_value!({"code": "FORBIDDEN"}),
    )
}

pub fn require_browser(context: &Context) -> juniper::FieldResult<()> {
    match context.credential {
        Credential::Session | Credential::TrustedHeader => Ok(()),
        Credential::ApiKey(_) => Err(forbidden()),
    }
}

pub async fn require_session(context: &Context) -> juniper::FieldResult<Uuid> {
    require_browser(context)?;
    require_user(context).await
}

fn selects_mutation(request: &juniper::http::GraphQLRequest, schema: &Schema) -> bool {
    use juniper::{Definition, OperationType};
    // Leave syntax and ambiguous/missing operation errors to Juniper. Such documents cannot execute.
    let Ok(document) = juniper::parser::parse_document_source(&request.query, &schema.schema)
    else {
        return false;
    };
    let mut operations = document.iter().filter_map(|definition| match definition {
        Definition::Operation(operation) => Some(&operation.item),
        Definition::Fragment(_) => None,
    });
    let selected = match request.operation_name.as_deref() {
        Some(name) => {
            operations.find(|operation| operation.name.as_ref().is_some_and(|n| n.item == name))
        }
        None => {
            let first = operations.next();
            if operations.next().is_none() {
                first
            } else {
                None
            }
        }
    };
    selected.is_some_and(|operation| operation.operation_type == OperationType::Mutation)
}

pub(crate) fn contains_mutation(request: &juniper_rocket::GraphQLRequest, schema: &Schema) -> bool {
    match request.as_ref() {
        juniper::http::GraphQLBatchRequest::Single(request) => selects_mutation(request, schema),
        juniper::http::GraphQLBatchRequest::Batch(requests) => requests
            .iter()
            .any(|request| selects_mutation(request, schema)),
    }
}
/// What a response should do with the session cookie.
#[derive(Debug, PartialEq, Eq)]
enum SessionCookie {
    /// The request signed somebody in; there was no cookie before it.
    Login(Uuid),
    /// An authenticated request, so the cookie is written again with a fresh week on it.
    Renew(Uuid),
    /// The request signed somebody out.
    Clear(Uuid),
    /// Nobody was signed in before or after, so there is nothing to say.
    Absent,
}

/// Rocket stamps `Expires` when a private cookie is written, and a browser never sends that
/// date back — so a cookie written once at login runs out exactly a week later, in the middle
/// of whatever the reader was doing, no matter how much they used the app in between. Writing
/// it again on every authenticated request is what turns the week into a sliding one. There is
/// no cheaper test for "is it about to expire": the request carries the value and nothing else.
fn session_cookie(before: Option<Uuid>, after: Option<Uuid>) -> SessionCookie {
    match (before, after) {
        (Some(_), Some(uid)) => SessionCookie::Renew(uid),
        (None, Some(uid)) => SessionCookie::Login(uid),
        (Some(uid), None) => SessionCookie::Clear(uid),
        (None, None) => SessionCookie::Absent,
    }
}

fn database_unavailable() -> juniper_rocket::GraphQLResponse {
    juniper_rocket::GraphQLResponse::error(juniper::FieldError::from("Database unavailable"))
}

pub async fn graphql_handler(
    pool: &rocket::State<crate::Pool>,
    config: &rocket::State<crate::config::Config>,
    cache: &rocket::State<Arc<AnalysisCache>>,
    cookies: &rocket::http::CookieJar<'_>,
    presented: Presented<'_>,
    request: juniper_rocket::GraphQLRequest,
    schema: &rocket::State<Schema>,
) -> juniper_rocket::GraphQLResponse {
    let (user_id, credential) = match presented {
        Presented::Authorization(header) => {
            let Some(token) = header.strip_prefix("Bearer ") else {
                return juniper_rocket::GraphQLResponse::error(unauthenticated());
            };
            let authenticated = match pool.get().await {
                Ok(mut conn) => crate::api_keys::authenticate(&mut conn, token).await,
                Err(_) => return database_unavailable(),
            };
            match authenticated {
                Ok(Some((id, access))) => (Some(id), Credential::ApiKey(access)),
                Ok(None) => return juniper_rocket::GraphQLResponse::error(unauthenticated()),
                Err(_) => return database_unavailable(),
            }
        }
        Presented::RemoteUser(name) => {
            let found = match pool.get().await {
                Ok(mut conn) => crate::queries::query_user_by_name(&mut conn, name).await,
                Err(_) => return database_unavailable(),
            };
            match found {
                Ok(Some(user)) => (Some(user.id), Credential::TrustedHeader),
                // nobody is created for a name the proxy knows and moneta does not
                Ok(None) => {
                    warn!("Remote-User '{name}' is not a user here");
                    (None, Credential::TrustedHeader)
                }
                Err(_) => return database_unavailable(),
            }
        }
        Presented::AmbiguousRemoteUser => {
            warn!("Refusing a request with more than one Remote-User header");
            (None, Credential::TrustedHeader)
        }
        Presented::Cookie(user_id) => (user_id, Credential::Session),
    };
    if credential == Credential::ApiKey(crate::api_keys::ApiKeyAccess::Read)
        && contains_mutation(&request, schema)
    {
        return juniper_rocket::GraphQLResponse::error(forbidden());
    }
    let user_id_writer = Arc::new(RwLock::new(user_id));

    let context = Context {
        config: (*config).clone(),
        pool: (*pool).clone(),
        user_id: user_id_writer.clone(),
        credential,
        cache: (*cache).clone(),
        wrote: AtomicBool::new(false),
        account_loader: get_account_loader((*pool).clone()),
        category_loader: get_category_loader((*pool).clone()),
        account_usage_loader: get_account_usage_loader((*pool).clone()),
        category_usage_loader: get_category_usage_loader((*pool).clone()),
        user_loader: get_user_loader((*pool).clone()),
    };

    let r = request.execute(schema, &context).await;

    if context.wrote.load(Ordering::Relaxed) {
        context.cache.clear();
    }

    let new_user_id = *user_id_writer.read().await;
    if credential != Credential::Session {
        return r;
    }
    match session_cookie(user_id, new_user_id) {
        SessionCookie::Login(uid) => {
            info!("Login for user_id {}", uid);
            cookies.add_private(rocket::http::Cookie::new("user_id", uid.to_string()));
        }
        // silent: this is every authenticated request, and a line each would drown the log
        SessionCookie::Renew(uid) => {
            cookies.add_private(rocket::http::Cookie::new("user_id", uid.to_string()));
        }
        SessionCookie::Clear(uid) => {
            info!("Logout for user_id {}", uid);
            cookies.remove_private(rocket::http::Cookie::build("user_id"));
        }
        SessionCookie::Absent => {}
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

    /// Browser-session credential management. API keys cannot list key metadata.
    async fn api_keys(context: &Context) -> juniper::FieldResult<ApiKeyQuery> {
        require_session(context).await?;
        Ok(ApiKeyQuery)
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

    /// Deliberately beside `analysis` rather than inside it: what a depot is worth today is
    /// read from renditier and never booked, so it must not reach the memoized analysis, the
    /// balances or anything monthly.
    fn investments() -> InvestmentQuery {
        InvestmentQuery
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
    fn users(context: &Context) -> juniper::FieldResult<UserMutation> {
        require_browser(context)?;
        Ok(UserMutation)
    }

    /// Credential writes do not affect financial analysis.
    async fn api_keys(context: &Context) -> juniper::FieldResult<ApiKeyMutation> {
        require_session(context).await?;
        Ok(ApiKeyMutation)
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
            credential: Credential::Session,
            cache: Arc::new(AnalysisCache::default()),
            wrote: AtomicBool::new(false),
            account_loader: get_account_loader(pool.clone()),
            category_loader: get_category_loader(pool.clone()),
            account_usage_loader: get_account_usage_loader(pool.clone()),
            category_usage_loader: get_category_usage_loader(pool.clone()),
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

    /// The renewal case is the one worth holding: without it the cookie is written once and
    /// the session ends a week after the login rather than a week after the last use.
    #[test]
    fn an_authenticated_request_renews_the_session() {
        let uid = Uuid::from_u128(1);
        assert_eq!(
            session_cookie(Some(uid), Some(uid)),
            SessionCookie::Renew(uid)
        );
        assert_eq!(session_cookie(None, Some(uid)), SessionCookie::Login(uid));
        assert_eq!(session_cookie(Some(uid), None), SessionCookie::Clear(uid));
        assert_eq!(session_cookie(None, None), SessionCookie::Absent);
    }
}
