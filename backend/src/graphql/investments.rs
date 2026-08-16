//! What the depots are worth right now, read from stockdb and never written down.
//!
//! A balance in moneta is a reading somebody took on purpose. For a depot that happens about
//! once a year, with a conservative haircut on the gains, and that is the point: the monthly
//! analysis is about spending, and a stock market moving those numbers around would ruin it as
//! an answer to that question. So the market value is an overlay and nothing else -- it is
//! never stored, never enters `simulate_balances` or the analysis SQL, and is offered only
//! beside quantities that are a stock of wealth right now.
//!
//! It is also its own query rather than a field on the analysis payload. That keeps the
//! memoized analysis keys in `cache.rs` untouched, leaves the toggle a matter for the
//! frontend, and lets a stockdb that is down cost one null instead of the dashboard.

use std::sync::LazyLock;
use std::time::Duration;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use juniper::{FieldResult, GraphQLObject, graphql_object};
use log::warn;
use serde::Deserialize;
use uuid::Uuid;

use crate::graphql::root::{Context, require_user};
use crate::models::Account;
use crate::schema::{accounts, users};

use super::accounts::GraphQLAccount;

/// One client for the process. Building one sets up a TLS stack, which is not something to do
/// per depot per page load.
static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

/// Only the parts of stockdb's portfolio this cares about; the positions, their ISINs and the
/// transactions behind them are deliberately not read. moneta has no model for any of that,
/// and a depot here is an account with a worth.
#[derive(Debug, Deserialize)]
struct StockdbPortfolio {
    invested: f64,
    value: Option<f64>,
    irr: Option<f64>,
    stocks: Vec<StockdbPosition>,
}

#[derive(Debug, Deserialize)]
struct StockdbPosition {
    data_source: Option<StockdbDataSource>,
}

#[derive(Debug, Deserialize)]
struct StockdbDataSource {
    price: StockdbPrice,
}

#[derive(Debug, Deserialize)]
struct StockdbPrice {
    date: DateTime<Utc>,
}

/// What a linked depot is worth, beside what has been booked for it.
///
/// Every figure is nullable and for one reason: stockdb could not answer, or had no price
/// recent enough to use. A reader is told that rather than shown a zero.
#[derive(Debug, Clone, GraphQLObject)]
#[graphql(name = "Investment", context = Context)]
pub struct GraphQLInvestment {
    /// The moneta account, so a caller can hold this against the balance it already has.
    pub account: GraphQLAccount,

    /// The depot in stockdb, which is also what a link into stockdb is built from.
    pub stockdb_account_id: i32,

    /// Market value at stockdb's latest usable price, in cents, scaled by the account's
    /// ownership fraction so that it compares to a rendered balance.
    pub value: Option<f64>,

    /// What went in, in cents: the net of every payment and fee, sign flipped, since stockdb
    /// counts a purchase as money leaving.
    pub invested: Option<f64>,

    /// Internal rate of return as a fraction, so 0.07 is seven percent.
    pub irr: Option<f64>,

    /// How old the oldest price behind `value` is. stockdb will use a price up to four days
    /// stale, so "realtime" needs a date next to it or it claims more than it knows.
    pub as_of: Option<DateTime<Utc>>,
}

/// A depot as stockdb lists it, which is all the account dialog needs to offer a choice.
#[derive(Debug, Clone, GraphQLObject, Deserialize)]
#[graphql(name = "StockdbAccount", context = Context)]
pub struct GraphQLStockdbAccount {
    pub id: i32,
    pub name: String,
}

pub struct InvestmentQuery;

#[graphql_object(
    Context = Context,
)]
impl InvestmentQuery {
    /// Where a browser reaches stockdb, so a linked depot can offer a way over to it. Null
    /// where none is configured, and deliberately not the url the backend itself calls: in the
    /// cluster that one is a Service name and resolves nowhere outside it.
    pub fn url(context: &Context) -> Option<String> {
        [
            context.config.stockdb.public_url.trim_end_matches('/'),
            context.config.stockdb.url.trim_end_matches('/'),
        ]
        .into_iter()
        .find(|u| !u.is_empty())
        .map(str::to_owned)
    }

    /// The depots this reader has over in stockdb, so the account dialog can offer a choice
    /// rather than ask for a number. Empty when there is no stockdb configured, no token, or
    /// nothing to reach — the dialog goes disabled on that rather than guessing why.
    pub async fn accounts(context: &Context) -> FieldResult<Vec<GraphQLStockdbAccount>> {
        let user_id = require_user(context).await?;

        let Some((base_url, token)) = credentials(context, user_id).await? else {
            return Ok(vec![]);
        };

        let timeout = Duration::from_secs(context.config.stockdb.timeout_seconds);

        Ok(
            get::<Vec<GraphQLStockdbAccount>>(&base_url, &token, "/api/accounts", timeout)
                .await
                .unwrap_or_default(),
        )
    }

    /// Every account of the reader's that is linked to a depot in stockdb. Empty when none is
    /// linked, or when this deployment has no stockdb configured.
    pub async fn list(context: &Context) -> FieldResult<Vec<GraphQLInvestment>> {
        let user_id = require_user(context).await?;

        // The connection is given back before the first request goes out. A page load holds one
        // per query as it is, and an HTTP call is a far longer await than a dataloader batch --
        // exactly the pattern that deadlocks a bounded pool.
        let linked: Vec<Account> = {
            let conn = &mut context.conn().await?;

            accounts::table
                .filter(accounts::user_id.eq(user_id))
                .filter(accounts::stockdb_account_id.is_not_null())
                .select(Account::as_select())
                .load(conn)
                .await?
        };

        let Some((base_url, token)) = credentials(context, user_id).await? else {
            if !linked.is_empty() {
                warn!(
                    "User {user_id} has {} account(s) linked to stockdb but no token or no \
                     configured url; `moneta user --stockdb-token` sets the former",
                    linked.len()
                );
            }

            return Ok(vec![]);
        };

        let base_url = base_url.as_str();
        let timeout = Duration::from_secs(context.config.stockdb.timeout_seconds);

        // Concurrently, so that two depots cost one round trip rather than two in a row.
        let fetched = futures::future::join_all(linked.into_iter().map(|account| {
            let token = token.clone();

            async move {
                let stockdb_account_id = account
                    .stockdb_account_id
                    .expect("filtered on stockdb_account_id being present");

                let path = format!("/api/accounts/{stockdb_account_id}/portfolio");
                let portfolio = get::<StockdbPortfolio>(base_url, &token, &path, timeout).await;
                (account, stockdb_account_id, portfolio)
            }
        }))
        .await;

        Ok(fetched
            .into_iter()
            .map(|(account, stockdb_account_id, portfolio)| {
                // A jointly held depot contributes the reader's share of itself, the same way
                // the rendered views scale its balance. Without this the two numbers on the
                // dashboard would be measuring different things.
                let share = account.ownership_fraction;

                // A portfolio with no positions in it is what stockdb answers for a depot that
                // is not there any more, or never was this reader's -- and it answers 0,00 EUR,
                // which is a wrong number rather than a missing one. A link left pointing at a
                // deleted depot should say nothing, not claim the money is gone.
                let portfolio = portfolio.filter(|p| !p.stocks.is_empty());

                GraphQLInvestment {
                    account: GraphQLAccount::new(account),
                    stockdb_account_id,
                    value: portfolio
                        .as_ref()
                        .and_then(|p| p.value)
                        .map(|v| v * 100.0 * share),
                    invested: portfolio.as_ref().map(|p| -p.invested * 100.0 * share),
                    irr: portfolio.as_ref().and_then(|p| p.irr),
                    as_of: portfolio.as_ref().and_then(|p| p.as_of()),
                }
            })
            .collect())
    }
}

impl StockdbPortfolio {
    /// The oldest price the value rests on. The pessimistic end on purpose: one stale position
    /// makes the total as old as it is, and saying otherwise would overstate the number.
    fn as_of(&self) -> Option<DateTime<Utc>> {
        self.stocks
            .iter()
            .filter_map(|s| s.data_source.as_ref().map(|d| d.price.date))
            .min()
    }
}

/// The base url and this reader's token, or `None` where either is missing -- which is what
/// every deployment without a stockdb looks like, and is not an error.
async fn credentials(context: &Context, user_id: Uuid) -> FieldResult<Option<(String, String)>> {
    let base_url = context.config.stockdb.url.trim_end_matches('/').to_owned();
    if base_url.is_empty() {
        return Ok(None);
    }

    let token: Option<String> = {
        let conn = &mut context.conn().await?;

        users::table
            .find(user_id)
            .select(users::stockdb_token)
            .first(conn)
            .await?
    };

    Ok(token.map(|token| (base_url, token)))
}

/// `None` for every way this can fail -- unreachable, refused, unparseable. Callers turn that
/// into a missing figure or an empty list, because a dashboard that cannot say what a depot is
/// worth today is still right about everything that was actually booked.
async fn get<T: serde::de::DeserializeOwned>(
    base_url: &str,
    token: &str,
    path: &str,
    timeout: Duration,
) -> Option<T> {
    let url = format!("{base_url}{path}");

    let response = CLIENT
        .get(&url)
        .bearer_auth(token)
        .timeout(timeout)
        .send()
        .await;

    let response = match response {
        Ok(response) if response.status().is_success() => response,
        Ok(response) => {
            warn!("stockdb answered {} for {}", response.status(), url);
            return None;
        }
        Err(e) => {
            warn!("Could not reach stockdb at {url}: {e}");
            return None;
        }
    };

    match response.json::<T>().await {
        Ok(parsed) => Some(parsed),
        Err(e) => {
            warn!("Could not read stockdb's answer for {url}: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape read out of stockdb's answer, pinned against a trimmed copy of a real one.
    /// Everything the resolver reports is derived here, and the two sign conventions -- money
    /// in counted negative there, positive here -- are exactly the kind of thing that inverts
    /// silently.
    #[test]
    fn reads_stockdbs_portfolio() {
        let body = r#"{
            "invested": -1507.5,
            "value": 1700.0,
            "irr": 0.07,
            "stocks": [
                {
                    "isin": "DE0001234567",
                    "data_source": {
                        "price": { "date": "2026-08-14T17:30:00Z", "price": 101.5 },
                        "exchange": {}
                    }
                },
                {
                    "isin": "DE0007654321",
                    "data_source": {
                        "price": { "date": "2026-08-16T09:00:00Z", "price": 42.0 },
                        "exchange": {}
                    }
                },
                { "isin": "DE0009999999", "data_source": null }
            ]
        }"#;

        let portfolio: StockdbPortfolio = serde_json::from_str(body).unwrap();

        assert_eq!(portfolio.value, Some(1700.0));
        assert_eq!(portfolio.irr, Some(0.07));
        assert_eq!(-portfolio.invested * 100.0, 150750.0);

        // the older of the two, not the newer and not the one without a price at all
        assert_eq!(
            portfolio.as_of(),
            Some("2026-08-14T17:30:00Z".parse::<DateTime<Utc>>().unwrap())
        );
    }

    /// A position stockdb could price nothing of leaves no date to report.
    #[test]
    fn a_portfolio_without_prices_has_no_date() {
        let portfolio: StockdbPortfolio = serde_json::from_str(
            r#"{ "invested": -100.0, "value": null, "irr": null,
                 "stocks": [{ "data_source": null }] }"#,
        )
        .unwrap();

        assert_eq!(portfolio.as_of(), None);
        assert_eq!(portfolio.value, None);
    }
}
