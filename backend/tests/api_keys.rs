//! Real HTTP authentication and credential lifecycle against disposable PostgreSQL 18.
//! Run after `scoping`, not concurrently against its fixture database.

use chrono::{DateTime, Utc};
use diesel::{
    prelude::*,
    sql_types::{BigInt, Jsonb, Nullable, Timestamptz},
};
use diesel_async::RunQueryDsl;
use rocket::{
    http::{ContentType, Header},
    local::asynchronous::Client,
};
use serde_json::{Value, json};

const MINE: &str = "00000000-0000-7000-8000-000000000001";
const THEIRS: &str = "00000000-0000-7000-8000-000000000002";
const MY_ACCOUNT: &str = "00000000-0000-7000-8000-0000000000a1";
const THEIR_ACCOUNT: &str = "00000000-0000-7000-8000-0000000000a2";
const EXPENSES: &str =
    "{ expenses { list { edges { node { id title effectiveAmount createdAt updatedAt } } } } }";
const KEY_LIST: &str =
    "{ apiKeys { list { id name prefix access createdAt expiresAt lastUsedAt } } }";

struct Reply {
    body: Value,
    cookies: bool,
}

async fn post(client: &Client, body: Value, authorization: Option<&str>) -> Reply {
    let mut request = client
        .post("/graphql")
        .header(ContentType::JSON)
        .body(body.to_string());
    if let Some(value) = authorization {
        request = request.header(Header::new("Authorization", value.to_owned()));
    }
    let response = request.dispatch().await;
    let cookies = response.headers().get("Set-Cookie").next().is_some();
    Reply {
        body: response.into_json().await.expect("JSON response"),
        cookies,
    }
}

async fn query(client: &Client, document: &str, token: Option<&str>) -> Reply {
    let authorization = token.map(|token| format!("Bearer {token}"));
    post(client, json!({"query": document}), authorization.as_deref()).await
}

fn success(reply: &Reply) {
    assert!(
        reply.body.get("errors").is_none(),
        "GraphQL errors: {}",
        reply.body["errors"]
    );
}
fn code(reply: &Reply, expected: &str) {
    assert_eq!(reply.body["errors"][0]["extensions"]["code"], expected);
}
async fn login(client: &Client, name: &str) {
    let result = post(client, json!({"query": "mutation($name: String!) { users { login(name: $name, password: \"dummy-password\") { id } } }", "variables": {"name": name}}), None).await;
    success(&result);
    assert!(result.cookies, "password login must set a cookie");
}

async fn create(
    client: &Client,
    name: &str,
    access: &str,
    expiry: Option<DateTime<Utc>>,
) -> (String, String) {
    let result = post(client, json!({"query": "mutation($name: String!, $access: ApiKeyAccess!, $expiry: DateTime) { apiKeys { create(name: $name, access: $access, expiresAt: $expiry) { token key { id name access expiresAt } } } }", "variables": {"name": name, "access": access, "expiry": expiry}}), None).await;
    success(&result);
    let created = &result.body["data"]["apiKeys"]["create"];
    (
        created["key"]["id"].as_str().expect("id").to_owned(),
        created["token"]
            .as_str()
            .expect("one-time token")
            .to_owned(),
    )
}

#[derive(QueryableByName)]
struct Snapshot {
    #[diesel(sql_type = Jsonb)]
    value: Value,
}
async fn snapshot(pool: &moneta::Pool) -> Value {
    diesel::sql_query("SELECT jsonb_build_object('users', (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM users t), 'accounts', (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM accounts t), 'balances', (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM balances t), 'expenses', (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM expenses t), 'changes', (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM expense_changes t), 'categories', (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM categories t), 'automation', (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM automation_rules t), 'periodic', (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM periodic_rules t), 'deliveries', (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM seen_deliveries t)) AS value")
        .get_result::<Snapshot>(&mut pool.get().await.unwrap()).await.unwrap().value
}
#[derive(QueryableByName)]
struct LastUsed {
    #[diesel(sql_type = Nullable<Timestamptz>)]
    last_used_at: Option<DateTime<Utc>>,
}
async fn last_used(pool: &moneta::Pool, id: &str) -> Option<DateTime<Utc>> {
    diesel::sql_query("SELECT last_used_at FROM api_keys WHERE id = $1::text::uuid")
        .bind::<diesel::sql_types::Text, _>(id)
        .get_result::<LastUsed>(&mut pool.get().await.unwrap())
        .await
        .unwrap()
        .last_used_at
}
async fn sql(pool: &moneta::Pool, statement: &str) {
    diesel::sql_query(statement)
        .execute(&mut pool.get().await.unwrap())
        .await
        .unwrap();
}

#[tokio::test]
async fn credentials_preserve_the_browser_and_owner_boundaries() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        assert!(
            std::env::var("MONETA_REQUIRE_DB_TESTS").is_err(),
            "TEST_DATABASE_URL is required"
        );
        eprintln!("TEST_DATABASE_URL unset: skipping real-handler API-key tests");
        return;
    };
    moneta::migrate(&url).await.unwrap();
    let pool = moneta::pool(&url, "moneta-api-key-test").unwrap();
    sql(&pool, "TRUNCATE users, accounts, categories, expenses, expense_changes, balances, seen_deliveries, automation_rules, periodic_rules CASCADE").await;
    let password = moneta::hash_password("dummy-password");
    for (id, name) in [(MINE, "dummy-alice"), (THEIRS, "dummy-bob")] {
        diesel::sql_query("INSERT INTO users (id, name, full_name, password_hash) VALUES ($1::text::uuid, $2, $2, $3)")
            .bind::<diesel::sql_types::Text, _>(id).bind::<diesel::sql_types::Text, _>(name).bind::<diesel::sql_types::Text, _>(&password)
            .execute(&mut pool.get().await.unwrap()).await.unwrap();
    }
    sql(&pool, &format!("INSERT INTO accounts (id,user_id,name,description,kind,availability,risk,opened,ownership_fraction) VALUES ('{MY_ACCOUNT}','{MINE}','Dummy checking','','debit','immediately','none','2026-01-01',1), ('{THEIR_ACCOUNT}','{THEIRS}','Other dummy checking','','debit','immediately','none','2026-01-01',1)")).await;
    sql(&pool, "INSERT INTO expenses (id,title,store,description,comments,accrual_start,accrual_end,is_template,is_tax_relevant,is_unchecked,is_incomplete,is_extraordinary,receipts,positions,created_at,updated_at) VALUES ('00000000-0000-7000-8000-0000000000e1','Dummy groceries','','','','2026-01-02','2026-01-02',false,false,false,false,false,'[]','[]',now(),now()), ('00000000-0000-7000-8000-0000000000e2','Other private groceries','','','','2026-01-02','2026-01-02',false,false,false,false,false,'[]','[]',now(),now())").await;
    sql(&pool, &format!("INSERT INTO expense_transactions (expense_id,idx,account_id,date,amount,comments,statement) VALUES ('00000000-0000-7000-8000-0000000000e1',0,'{MY_ACCOUNT}','2026-01-02',-100,'',''), ('00000000-0000-7000-8000-0000000000e2',0,'{THEIR_ACCOUNT}','2026-01-02',-200,'','')")).await;
    sql(&pool, "UPDATE expenses SET created_at = '2025-11-03T12:34:56Z', updated_at = '2026-04-05T16:17:18Z' WHERE id = '00000000-0000-7000-8000-0000000000e1'").await;

    let config = moneta::config::Config {
        verbosity: -2,
        ..Default::default()
    };
    let browser = Client::tracked(moneta::graphql::rocket(config.clone(), pool.clone()))
        .await
        .unwrap();
    let other = Client::tracked(moneta::graphql::rocket(config.clone(), pool.clone()))
        .await
        .unwrap();
    let machine = Client::untracked(moneta::graphql::rocket(config, pool.clone()))
        .await
        .unwrap();
    login(&browser, "dummy-alice").await;
    login(&other, "dummy-bob").await;
    let session = query(&browser, EXPENSES, None).await;
    success(&session);
    assert!(
        session.cookies,
        "authenticated browser reads renew the session"
    );
    assert_eq!(
        session.body["data"]["expenses"]["list"]["edges"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let cache = browser
        .rocket()
        .state::<std::sync::Arc<moneta::graphql::cache::AnalysisCache>>()
        .unwrap();
    moneta::graphql::cache::cached(cache, "credential-write-regression".into(), async { Ok(7) })
        .await
        .unwrap();
    let (read_id, read) = create(&browser, " Ferret dummy ", "READ", None).await;
    let (write_id, write) = create(&browser, "Writer dummy", "WRITE", None).await;
    let (_, other_token) = create(&other, "Bob dummy", "READ", None).await;
    for variables in [
        json!({"name": " \t\n", "access": "READ", "expiry": null}),
        json!({"name": "Past", "access": "READ", "expiry": "2000-01-01T00:00:00Z"}),
        json!({"name": "Unknown access", "access": "ADMIN", "expiry": null}),
    ] {
        let rejected = post(&browser, json!({"query": "mutation($name: String!, $access: ApiKeyAccess!, $expiry: DateTime) { apiKeys { create(name: $name, access: $access, expiresAt: $expiry) { token } } }", "variables": variables}), None).await;
        assert!(rejected.body["errors"].is_array());
    }
    assert!(read.starts_with("mk_") && read.len() == 55);
    let metadata = query(&browser, KEY_LIST, None).await;
    success(&metadata);
    assert_eq!(
        metadata.body["data"]["apiKeys"]["list"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(!metadata.body.to_string().contains(&read));
    assert!(!metadata.body.to_string().contains("secretHash"));
    assert_eq!(last_used(&pool, &read_id).await, None);
    let bearer_read = query(&machine, EXPENSES, Some(&read)).await;
    success(&bearer_read);
    assert!(!bearer_read.cookies);
    assert_eq!(bearer_read.body, session.body);
    let expense = &bearer_read.body["data"]["expenses"]["list"]["edges"][0]["node"];
    assert_eq!(
        expense["createdAt"]
            .as_str()
            .unwrap()
            .parse::<DateTime<Utc>>()
            .unwrap(),
        "2025-11-03T12:34:56Z".parse::<DateTime<Utc>>().unwrap()
    );
    assert_eq!(
        expense["updatedAt"]
            .as_str()
            .unwrap()
            .parse::<DateTime<Utc>>()
            .unwrap(),
        "2026-04-05T16:17:18Z".parse::<DateTime<Utc>>().unwrap()
    );
    assert_ne!(
        query(&machine, EXPENSES, Some(&other_token)).await.body,
        session.body
    );
    assert!(last_used(&pool, &read_id).await.is_some());

    // Every mutation namespace, not just a convenient expense write. The selected AST operation
    // is what matters, regardless of aliases, fragments, operation names or batching.
    let before = snapshot(&pool).await;
    let mutations = [
        format!("mutation ReadExpenses {{ accounts {{ delete(id: \"{MY_ACCOUNT}\") }} }}"),
        "mutation { balances { delete(id: \"00000000-0000-7000-8000-0000000000b1\") } }".into(),
        "mutation { renamed: expenses { delete(id: \"00000000-0000-7000-8000-0000000000e1\") } }".into(),
        "mutation { categories { create(category: {name: \"Should not exist\", description: \"\"}) { id } } }".into(),
        "mutation { automationRules { delete(id: \"00000000-0000-7000-8000-000000000001\") } }".into(),
        "mutation { periodicRules { run } }".into(),
        "mutation { deliveries { submit(deliveries: []) { status } } }".into(),
        "mutation { ...Hidden } fragment Hidden on Mutation { users { visit { current } } }".into(),
        "mutation { apiKeys { create(name: \"forbidden\", access: WRITE) { token } } }".into(),
        "query Safe { apiVersion } mutation Harmless { users { logout } }".into(),
    ];
    for mutation in mutations {
        let result = post(&machine, json!({"query": mutation, "operationName": if mutation.starts_with("query Safe") { Some("Harmless") } else { None }}), Some(&format!("Bearer {read}"))).await;
        code(&result, "FORBIDDEN");
        assert!(!result.cookies);
    }
    let selected_query = post(&machine, json!({"query": "query MutationNamedQuery { apiVersion } mutation Other { users { logout } }", "operationName": "MutationNamedQuery"}), Some(&format!("Bearer {read}"))).await;
    success(&selected_query);
    let batch = post(
        &machine,
        json!([{ "query": "{ apiVersion }" }, { "query": "mutation { users { logout } }" }]),
        Some(&format!("Bearer {read}")),
    )
    .await;
    code(&batch, "FORBIDDEN");
    assert_eq!(
        before,
        snapshot(&pool).await,
        "read-only mutations changed domain state"
    );

    for token in [&read, &write] {
        for document in [KEY_LIST.to_owned(), "mutation { users { login(name: \"dummy-bob\", password: \"dummy-password\") { id } } }".into(), "mutation { users { logout } }".into(), "mutation { users { visit { current } } }".into(), "mutation { apiKeys { create(name: \"No\", access: READ) { token } } }".into(), format!("mutation {{ apiKeys {{ revoke(id: \"{read_id}\") }} }}")] {
            let result = query(&browser, &document, Some(token)).await;
            code(&result, "FORBIDDEN");
            assert!(!result.cookies, "bearer must never modify a valid browser cookie");
        }
    }
    assert_eq!(before, snapshot(&pool).await);
    assert_eq!(
        moneta::graphql::cache::cached(cache, "credential-write-regression".into(), async {
            Ok(9)
        })
        .await
        .unwrap(),
        7,
        "credential writes/authentication cleared financial cache"
    );

    // A write credential has exactly the session's ordinary domain authorization.
    let balance_input = |account: &str, amount: i32| json!({"accountId": account, "date": "2026-01-03T12:00:00Z", "amount": amount, "comments": "Dummy balance"});
    let create_balance = "mutation($balance: BalanceInput!) { balances { create(balance: $balance) { id amount } } }";
    let added = post(
        &machine,
        json!({"query": create_balance, "variables": {"balance": balance_input(MY_ACCOUNT, 100)}}),
        Some(&format!("Bearer {write}")),
    )
    .await;
    success(&added);
    let id = added.body["data"]["balances"]["create"]["id"]
        .as_str()
        .unwrap();
    let updated = post(&machine, json!({"query": "mutation($id: UUID!, $balance: BalanceInput!) { balances { update(id: $id, balance: $balance) { amount } } }", "variables": {"id": id, "balance": balance_input(MY_ACCOUNT, 200)}}), Some(&format!("Bearer {write}"))).await;
    success(&updated);
    assert_eq!(updated.body["data"]["balances"]["update"]["amount"], 200);
    let other_write = post(&machine, json!({"query": create_balance, "variables": {"balance": balance_input(THEIR_ACCOUNT, 999)}}), Some(&format!("Bearer {write}"))).await;
    assert!(other_write.body["errors"].is_array());
    success(
        &query(
            &machine,
            &format!("mutation {{ balances {{ delete(id: \"{id}\") }} }}"),
            Some(&write),
        )
        .await,
    );
    let protected_before = snapshot(&pool).await;
    for document in [
        "mutation { expenses { delete(id: \"00000000-0000-7000-8000-0000000000e2\") } }",
        "mutation { accounts { delete(id: \"00000000-0000-7000-8000-0000000000a2\") } }",
    ] {
        assert!(query(&machine, document, Some(&write)).await.body["errors"].is_array());
    }
    assert_eq!(
        protected_before,
        snapshot(&pool).await,
        "other-owner write changed domain state"
    );
    let mixed = query(&machine, &format!("mutation {{ balances {{ create(balance: {{ accountId: \"{MY_ACCOUNT}\", date: \"2026-01-03T12:00:00Z\", amount: 300, comments: \"Mixed permitted field\" }}) {{ id }} }} users {{ logout }} }}"), Some(&write)).await;
    code(&mixed, "FORBIDDEN");
    assert_ne!(
        protected_before,
        snapshot(&pool).await,
        "permitted earlier fields may commit before session-only rejection"
    );
    sql(
        &pool,
        "DELETE FROM balances WHERE comments = 'Mixed permitted field'",
    )
    .await;
    assert_eq!(
        query(&machine, EXPENSES, Some(&write)).await.body,
        session.body
    );

    // Explicit Authorization never falls back to the valid cookie.
    let suffix = read.split_once('.').unwrap().1;
    let wrong_secret = format!(
        "{}.{}",
        read.split_once('.').unwrap().0,
        if suffix.starts_with('A') {
            format!("B{}", "A".repeat(42))
        } else {
            "A".repeat(43)
        }
    );
    let unknown = format!("mk_ffffffff.{suffix}");
    for header in [
        "".to_owned(),
        "Basic bad".to_owned(),
        "Bearer malformed".to_owned(),
        format!("Bearer {wrong_secret}"),
        format!("Bearer {unknown}"),
        format!("Bearer {read} extra"),
    ] {
        let result = post(&browser, json!({"query": "{ apiVersion }"}), Some(&header)).await;
        code(&result, "UNAUTHENTICATED");
        assert!(!result.cookies);
        assert!(!result.body.to_string().contains(&read));
    }
    let get_response = machine
        .get("/graphql?query=%7Busers%7Bme%7Bname%7D%7D%7D")
        .header(Header::new("Authorization", format!("Bearer {read}")))
        .dispatch()
        .await;
    assert!(get_response.headers().get("Set-Cookie").next().is_none());
    let get_body: Value = get_response.into_json().await.unwrap();
    assert_eq!(get_body["data"]["users"]["me"]["name"], "dummy-alice");
    let denied_get = machine
        .get("/graphql?query=mutation%20%7Busers%7Blogout%7D%7D")
        .header(Header::new("Authorization", format!("Bearer {read}")))
        .dispatch()
        .await;
    let denied_get: Value = denied_get.into_json().await.unwrap();
    assert_eq!(denied_get["errors"][0]["extensions"]["code"], "FORBIDDEN");

    // Successful authentication counts even if parsing, validation or permission checks fail.
    sql(
        &pool,
        &format!("UPDATE api_keys SET last_used_at = NULL WHERE id = '{read_id}'"),
    )
    .await;
    code(
        &query(&machine, "mutation { users { logout } }", Some(&read)).await,
        "FORBIDDEN",
    );
    let first_use = last_used(&pool, &read_id).await.unwrap();
    let requests = (0..24).map(|_| query(&machine, "{ invalidField }", Some(&read)));
    for result in futures::future::join_all(requests).await {
        assert!(result.body["errors"].is_array());
    }
    assert_eq!(last_used(&pool, &read_id).await, Some(first_use));
    sql(
        &pool,
        &format!(
            "UPDATE api_keys SET last_used_at = now() - interval '2 minutes' WHERE id = '{read_id}'"
        ),
    )
    .await;
    query(&machine, "broken graphql", Some(&read)).await;
    assert!(last_used(&pool, &read_id).await.unwrap() >= first_use);
    sql(
        &pool,
        &format!(
            "UPDATE api_keys SET last_used_at = now() - interval '2 minutes' WHERE id = '{read_id}'"
        ),
    )
    .await;
    sql(
        &pool,
        "CREATE TABLE api_key_test_updates (key_id uuid NOT NULL)",
    )
    .await;
    sql(&pool, "CREATE FUNCTION api_key_test_count_update() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN INSERT INTO api_key_test_updates VALUES (NEW.id); RETURN NEW; END $$").await;
    sql(&pool, "CREATE TRIGGER api_key_test_updates AFTER UPDATE OF last_used_at ON api_keys FOR EACH ROW EXECUTE FUNCTION api_key_test_count_update()").await;
    let concurrent = (0..24).map(|_| query(&machine, "{ apiVersion }", Some(&read)));
    for result in futures::future::join_all(concurrent).await {
        success(&result);
    }
    let writes = diesel::sql_query("SELECT count(*) AS count FROM api_key_test_updates")
        .get_result::<Count>(&mut pool.get().await.unwrap())
        .await
        .unwrap();
    assert_eq!(
        writes.count, 1,
        "concurrent requests wrote last-used more than once"
    );
    sql(&pool, "DROP TRIGGER api_key_test_updates ON api_keys").await;
    sql(&pool, "DROP FUNCTION api_key_test_count_update()").await;
    sql(&pool, "DROP TABLE api_key_test_updates").await;

    let (expired_id, expired) = create(
        &browser,
        "Expired dummy",
        "READ",
        Some(Utc::now() + chrono::Duration::hours(1)),
    )
    .await;
    sql(&pool, &format!("UPDATE api_keys SET created_at = now() - interval '2 hours', expires_at = now() - interval '1 hour' WHERE id = '{expired_id}'")).await;
    code(
        &query(&machine, EXPENSES, Some(&expired)).await,
        "UNAUTHENTICATED",
    );
    assert_eq!(last_used(&pool, &expired_id).await, None);
    assert!(
        query(&browser, KEY_LIST, None).await.body["data"]["apiKeys"]["list"]
            .as_array()
            .unwrap()
            .iter()
            .any(|key| key["id"] == expired_id)
    );
    let attempt = query(
        &other,
        &format!("mutation {{ apiKeys {{ revoke(id: \"{read_id}\") }} }}"),
        None,
    )
    .await;
    success(&attempt);
    assert_eq!(attempt.body["data"]["apiKeys"]["revoke"], false);
    success(&query(&machine, EXPENSES, Some(&read)).await);
    success(
        &query(
            &browser,
            &format!("mutation {{ apiKeys {{ revoke(id: \"{read_id}\") }} }}"),
            None,
        )
        .await,
    );
    code(
        &query(&machine, EXPENSES, Some(&read)).await,
        "UNAUTHENTICATED",
    );
    assert!(
        !query(&browser, KEY_LIST, None)
            .await
            .body
            .to_string()
            .contains(&read_id)
    );

    // Password changes do not revoke independent API credentials.
    sql(
        &pool,
        &format!(
            "UPDATE users SET password_hash = 'deliberately invalid dummy hash' WHERE id = '{MINE}'"
        ),
    )
    .await;
    success(&query(&machine, EXPENSES, Some(&write)).await);
    let rows = diesel::sql_query("SELECT to_jsonb(k) AS value FROM api_keys k")
        .load::<Snapshot>(&mut pool.get().await.unwrap())
        .await
        .unwrap();
    for row in rows {
        assert!(!row.value.to_string().contains(&write));
        assert!(
            !row.value
                .to_string()
                .contains(write.split_once('.').unwrap().1)
        );
    }
    let logged_out = query(&browser, "mutation { users { logout } }", None).await;
    success(&logged_out);
    assert!(logged_out.cookies);
    code(&query(&browser, EXPENSES, None).await, "UNAUTHENTICATED");

    // A user without a password, as `moneta import` leaves them, cannot sign in with any.
    sql(
        &pool,
        &format!("UPDATE users SET password_hash = NULL WHERE id = '{MINE}'"),
    )
    .await;
    let refused = post(&browser, json!({"query": "mutation { users { login(name: \"dummy-alice\", password: \"dummy-password\") { id } } }"}), None).await;
    assert_eq!(refused.body["errors"][0]["message"], "Unauthorized");
    assert!(!refused.cookies, "a refused login must not set a cookie");

    // No financial references for this disposable third user: exercise the real owner FK cascade.
    sql(&pool, "INSERT INTO users (id,name,full_name,password_hash) VALUES ('00000000-0000-7000-8000-000000000003','deleted-dummy','Deleted dummy','x')").await;
    let orphan_token = moneta::api_keys::create(
        &mut pool.get().await.unwrap(),
        "00000000-0000-7000-8000-000000000003".parse().unwrap(),
        "Owner deletion",
        moneta::api_keys::ApiKeyAccess::Read,
        None,
    )
    .await
    .unwrap()
    .token;
    sql(&pool, "DELETE FROM users WHERE name = 'deleted-dummy'").await;
    code(
        &query(&machine, EXPENSES, Some(&orphan_token)).await,
        "UNAUTHENTICATED",
    );
    #[derive(QueryableByName)]
    struct Count {
        #[diesel(sql_type = BigInt)]
        count: i64,
    }
    let remaining = diesel::sql_query("SELECT count(*) FROM api_keys WHERE id = $1::text::uuid")
        .bind::<diesel::sql_types::Text, _>(&write_id)
        .get_result::<Count>(&mut pool.get().await.unwrap())
        .await
        .unwrap();
    assert_eq!(remaining.count, 1);
}
