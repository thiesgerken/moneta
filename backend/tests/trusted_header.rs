//! `Remote-User` from a trusted proxy, and what keeps the GraphQL endpoint out of reach of
//! requests another site can make with the reader's cookies, against the real HTTP handler.
//!
//! The identity half needs an empty database of its own, handed in as `TEST_DATABASE_URL`, and
//! truncates as it goes; run it after `backup`, not concurrently with the other suites.

use std::net::SocketAddr;

use diesel_async::{RunQueryDsl, SimpleAsyncConnection};
use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::{Client, LocalRequest};
use serde_json::{Value, json};

const ME: &str = "{ users { me { name } } }";
const KEYS: &str = "{ apiKeys { list { id } } }";

const PROXY: &str = "10.42.0.7:40000";
const OUTSIDE: &str = "192.0.2.1:40000";

fn config(trusted_header_authentication: bool) -> moneta::config::Config {
    let mut config = moneta::config::Config {
        verbosity: -2,
        ..Default::default()
    };
    config.api.trusted_header_authentication = trusted_header_authentication;
    config.api.trusted_proxies = vec!["10.42.0.0/16".parse().unwrap()];
    config
}

async fn client(config: moneta::config::Config, pool: &moneta::Pool) -> Client {
    Client::tracked(moneta::graphql::rocket(config, pool.clone()))
        .await
        .unwrap()
}

fn graphql<'c>(client: &'c Client, document: &str) -> LocalRequest<'c> {
    client
        .post("/graphql")
        .header(ContentType::JSON)
        .body(json!({ "query": document }).to_string())
}

fn from<'c>(request: LocalRequest<'c>, peer: &str) -> LocalRequest<'c> {
    request.remote(peer.parse::<SocketAddr>().unwrap())
}

fn remote_user<'c>(request: LocalRequest<'c>, name: &str) -> LocalRequest<'c> {
    request.header(Header::new("Remote-User", name.to_owned()))
}

struct Reply {
    body: Value,
    cookies: bool,
}

async fn send(request: LocalRequest<'_>) -> Reply {
    let response = request.dispatch().await;
    let cookies = response.headers().get("Set-Cookie").next().is_some();
    Reply {
        body: response.into_json().await.expect("JSON response"),
        cookies,
    }
}

fn me(reply: &Reply) -> &Value {
    &reply.body["data"]["users"]["me"]["name"]
}

fn code(reply: &Reply) -> &Value {
    &reply.body["errors"][0]["extensions"]["code"]
}

/// One test rather than several: they share one database, and a second test truncating it
/// while the first is reading would be a race with a very confusing failure.
#[tokio::test]
async fn remote_user_is_believed_from_trusted_proxies_only() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        assert!(
            std::env::var("MONETA_REQUIRE_DB_TESTS").is_err(),
            "TEST_DATABASE_URL is required"
        );
        eprintln!("TEST_DATABASE_URL is not set -- skipping the Remote-User tests");
        return;
    };

    moneta::migrate(&url).await.expect("migrations");
    let pool = moneta::pool(&url, "moneta-trusted-header-test").expect("pool");
    let password = moneta::hash_password("dummy-password");
    pool.get()
        .await
        .unwrap()
        .batch_execute(&format!(
            "TRUNCATE users, accounts, categories, expenses, expense_changes, balances, \
             seen_deliveries, automation_rules, periodic_rules, api_keys CASCADE;
             INSERT INTO users (name, full_name, password_hash) VALUES
                 ('rudolph', 'Rudolph', NULL),
                 ('kroesus', 'Krösus', '{password}');"
        ))
        .await
        .expect("fixtures");

    // --- off unless switched on ------------------------------------------------------------

    let off = client(config(false), &pool).await;
    let reply = send(remote_user(from(graphql(&off, ME), PROXY), "rudolph")).await;
    assert_eq!(code(&reply), "UNAUTHENTICATED", "{}", reply.body);

    // --- from a trusted proxy ---------------------------------------------------------------

    let on = client(config(true), &pool).await;

    // rudolph has no password at all, which is what an imported user looks like
    let reply = send(remote_user(from(graphql(&on, ME), PROXY), "rudolph")).await;
    assert_eq!(me(&reply), "rudolph", "{}", reply.body);
    assert!(!reply.cookies, "the proxy keeps the session, not a cookie");

    // a dual-stack listener reports an IPv4 proxy as an IPv4-mapped IPv6 address
    let reply = send(remote_user(
        from(graphql(&on, ME), "[::ffff:10.42.0.7]:40000"),
        "rudolph",
    ))
    .await;
    assert_eq!(me(&reply), "rudolph", "{}", reply.body);

    // it is a browser, so what only a session may do is open to it
    let reply = send(remote_user(from(graphql(&on, KEYS), PROXY), "rudolph")).await;
    assert!(reply.body.get("errors").is_none(), "{}", reply.body);

    let reply = send(remote_user(from(graphql(&on, ME), PROXY), "nobody")).await;
    assert_eq!(code(&reply), "UNAUTHENTICATED", "an unknown name is nobody");

    let reply = send(remote_user(
        remote_user(from(graphql(&on, ME), PROXY), "kroesus"),
        "rudolph",
    ))
    .await;
    assert_eq!(code(&reply), "UNAUTHENTICATED", "two headers are none");

    // --- from anywhere else -----------------------------------------------------------------

    let reply = send(remote_user(from(graphql(&on, ME), OUTSIDE), "rudolph")).await;
    assert_eq!(code(&reply), "UNAUTHENTICATED", "{}", reply.body);

    // what the caller writes about its own address changes nothing
    let reply = send(
        remote_user(from(graphql(&on, ME), OUTSIDE), "rudolph")
            .header(Header::new("X-Real-IP", "10.42.0.7"))
            .header(Header::new("X-Forwarded-For", "10.42.0.7")),
    )
    .await;
    assert_eq!(code(&reply), "UNAUTHENTICATED", "{}", reply.body);

    let reply = send(remote_user(graphql(&on, ME), "rudolph")).await;
    assert_eq!(
        code(&reply),
        "UNAUTHENTICATED",
        "no peer address is no proxy"
    );

    // --- against the other credentials ------------------------------------------------------

    // kroesus signs in with his password from outside the proxy, so the client holds his cookie
    let login =
        "mutation { users { login(name: \"kroesus\", password: \"dummy-password\") { id } } }";
    let reply = send(from(graphql(&on, login), OUTSIDE)).await;
    assert!(reply.cookies, "{}", reply.body);
    assert_eq!(me(&send(from(graphql(&on, ME), OUTSIDE)).await), "kroesus");

    // the proxy outvotes that cookie, and leaves it alone
    let reply = send(remote_user(from(graphql(&on, ME), PROXY), "rudolph")).await;
    assert_eq!(me(&reply), "rudolph", "{}", reply.body);
    assert!(!reply.cookies);
    assert_eq!(me(&send(from(graphql(&on, ME), OUTSIDE)).await), "kroesus");

    // ... and is ignored from outside, where the cookie still counts
    let reply = send(remote_user(from(graphql(&on, ME), OUTSIDE), "rudolph")).await;
    assert_eq!(me(&reply), "kroesus", "{}", reply.body);

    // an explicit Authorization header wins over the proxy, even when it is no good
    for header in ["Bearer malformed", "Basic nope", ""] {
        let reply = send(
            remote_user(from(graphql(&on, ME), PROXY), "rudolph")
                .header(Header::new("Authorization", header)),
        )
        .await;
        assert_eq!(
            code(&reply),
            "UNAUTHENTICATED",
            "{header:?}: {}",
            reply.body
        );
    }

    // and a valid key is its owner's, with what a key may do rather than what a browser may
    let kroesus = moneta::queries::query_user_by_name(&mut pool.get().await.unwrap(), "kroesus")
        .await
        .unwrap()
        .unwrap();
    let token = moneta::api_keys::create(
        &mut pool.get().await.unwrap(),
        kroesus.id,
        "Ferret",
        moneta::api_keys::ApiKeyAccess::Read,
        None,
    )
    .await
    .unwrap()
    .token;
    let keyed = |document: &str| {
        remote_user(from(graphql(&on, document), PROXY), "rudolph")
            .header(Header::new("Authorization", format!("Bearer {token}")))
    };
    assert_eq!(me(&send(keyed(ME)).await), "kroesus");
    assert_eq!(code(&send(keyed(KEYS)).await), "FORBIDDEN");

    // keep the rows countable for whoever looks at a failure
    let users: i64 = {
        #[derive(diesel::QueryableByName)]
        struct Count {
            #[diesel(sql_type = diesel::sql_types::BigInt)]
            count: i64,
        }
        diesel::sql_query("SELECT count(*) AS count FROM users")
            .get_result::<Count>(&mut pool.get().await.unwrap())
            .await
            .unwrap()
            .count
    };
    assert_eq!(users, 2, "a Remote-User must never create a user");
}

/// Needs no database: every request here is refused, or reads nothing but the API version.
#[tokio::test]
async fn only_json_posts_and_query_gets_reach_graphql_and_only_from_its_own_origin() {
    // never connected to; deadpool only dials when a connection is taken
    let pool = moneta::pool("postgres://nobody@127.0.0.1:1/nothing", "test").unwrap();
    let client = client(config(false), &pool).await;
    let query = json!({ "query": "{ apiVersion }" }).to_string();

    let response = client
        .post("/graphql")
        .header(ContentType::JSON)
        .body(&query)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // what an HTML form or a CORS-simple fetch can send without a preflight
    for content_type in [ContentType::Form, ContentType::Plain, ContentType::FormData] {
        let response = client
            .post("/graphql")
            .header(content_type.clone())
            .body(&query)
            .dispatch()
            .await;
        assert_ne!(response.status(), Status::Ok, "{content_type}");
    }
    let response = client.post("/graphql").body(&query).dispatch().await;
    assert_ne!(response.status(), Status::Ok, "no content type");

    // no other origin is let in: a preflight is not approved, and an answer is not readable
    let origin = Header::new("Origin", "https://elsewhere.example");
    let response = client
        .options("/graphql")
        .header(origin.clone())
        .header(Header::new("Access-Control-Request-Method", "POST"))
        .header(Header::new(
            "Access-Control-Request-Headers",
            "content-type",
        ))
        .dispatch()
        .await;
    assert!(
        response
            .headers()
            .get_one("Access-Control-Allow-Origin")
            .is_none(),
        "a cross-origin preflight was approved"
    );
    let response = client
        .post("/graphql")
        .header(ContentType::JSON)
        .header(origin)
        .body(&query)
        .dispatch()
        .await;
    assert!(
        response
            .headers()
            .get_one("Access-Control-Allow-Origin")
            .is_none()
    );
    assert!(
        response
            .headers()
            .get_one("Access-Control-Allow-Credentials")
            .is_none()
    );

    // a query over GET is fine, a mutation is not
    let response = client
        .get("/graphql?query=%7BapiVersion%7D")
        .dispatch()
        .await;
    let body: Value = response.into_json().await.unwrap();
    assert_eq!(body["data"]["apiVersion"], "1.0");

    let response = client
        .get("/graphql?query=mutation%20%7Busers%7Blogout%7D%7D")
        .dispatch()
        .await;
    let body: Value = response.into_json().await.unwrap();
    assert_eq!(
        body["errors"][0]["message"],
        "Mutations are only accepted over POST"
    );
    assert!(body["data"].is_null(), "{body}");
}
