//! User-owned credentials. Only the creation response ever contains the secret.

use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use diesel::{
    QueryableByName,
    sql_types::{Binary, Nullable, Text, Timestamptz, Uuid as SqlUuid},
};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, juniper::GraphQLEnum)]
pub enum ApiKeyAccess {
    /// Authenticated queries only. Sufficient for Ferret.
    Read,
    /// Queries and domain mutations, with the owner's ordinary permissions.
    Write,
}

impl ApiKeyAccess {
    fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "read" => Self::Read,
            "write" => Self::Write,
            _ => panic!("invalid API key access level in database"),
        }
    }
}

/// Non-secret metadata. Expired keys remain here until their owner removes them.
#[derive(QueryableByName)]
pub struct ApiKey {
    #[diesel(sql_type = SqlUuid)]
    pub id: Uuid,
    #[diesel(sql_type = Text)]
    pub name: String,
    #[diesel(sql_type = Text)]
    pub prefix: String,
    #[diesel(sql_type = Text)]
    access_level: String,
    #[diesel(sql_type = Timestamptz)]
    pub created_at: DateTime<Utc>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub expires_at: Option<DateTime<Utc>>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub last_used_at: Option<DateTime<Utc>>,
}

// Keep the database text representation out of the public API.
#[juniper::graphql_object(context = crate::graphql::root::Context)]
impl ApiKey {
    fn id(&self) -> Uuid {
        self.id
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn prefix(&self) -> &str {
        &self.prefix
    }
    fn access(&self) -> ApiKeyAccess {
        ApiKeyAccess::parse(&self.access_level)
    }
    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires_at
    }
    fn last_used_at(&self) -> Option<DateTime<Utc>> {
        self.last_used_at
    }
}

#[derive(juniper::GraphQLObject)]
#[graphql(context = crate::graphql::root::Context)]
pub struct CreatedApiKey {
    pub key: ApiKey,
    /// Copy this now. It cannot be recovered or returned again.
    pub token: String,
}

pub async fn list(
    conn: &mut AsyncPgConnection,
    user_id: Uuid,
) -> Result<Vec<ApiKey>, diesel::result::Error> {
    diesel::sql_query("SELECT id, name, prefix, access_level, created_at, expires_at, last_used_at FROM api_keys WHERE user_id = $1 ORDER BY created_at DESC, id DESC")
        .bind::<SqlUuid, _>(user_id)
        .load::<ApiKey>(conn)
        .await
}

pub async fn create(
    conn: &mut AsyncPgConnection,
    user_id: Uuid,
    name: &str,
    access: ApiKeyAccess,
    expires_at: Option<DateTime<Utc>>,
) -> juniper::FieldResult<CreatedApiKey> {
    let name = name.trim();
    if name.is_empty() {
        return Err("A key needs a name".into());
    }
    if expires_at.is_some_and(|expiry| expiry <= Utc::now()) {
        return Err("The expiry must be in the future".into());
    }
    let mut prefix_bytes = [0; 4];
    let mut secret_bytes = [0; 32];
    getrandom::fill(&mut prefix_bytes).expect("OS randomness unavailable");
    getrandom::fill(&mut secret_bytes).expect("OS randomness unavailable");
    let prefix = format!("{:08x}", u32::from_be_bytes(prefix_bytes));
    let secret = BASE64_URL_SAFE_NO_PAD.encode(secret_bytes);
    let hash = Sha256::digest(secret.as_bytes());
    // A prefix collision fails the unique constraint, never replaces a credential.
    let row = diesel::sql_query("INSERT INTO api_keys (user_id, name, prefix, secret_hash, access_level, expires_at) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id, name, prefix, access_level, created_at, expires_at, last_used_at")
        .bind::<SqlUuid, _>(user_id)
        .bind::<Text, _>(name)
        .bind::<Text, _>(&prefix)
        .bind::<Binary, _>(hash.as_slice())
        .bind::<Text, _>(access.as_str())
        .bind::<Nullable<Timestamptz>, _>(expires_at)
        .get_result::<ApiKey>(conn).await?;
    Ok(CreatedApiKey {
        key: row,
        token: format!("mk_{prefix}.{secret}"),
    })
}

pub async fn revoke(
    conn: &mut AsyncPgConnection,
    user_id: Uuid,
    id: Uuid,
) -> Result<bool, diesel::result::Error> {
    Ok(
        diesel::sql_query("DELETE FROM api_keys WHERE id = $1 AND user_id = $2")
            .bind::<SqlUuid, _>(id)
            .bind::<SqlUuid, _>(user_id)
            .execute(conn)
            .await?
            == 1,
    )
}

#[derive(QueryableByName)]
struct AuthenticationRow {
    #[diesel(sql_type = SqlUuid)]
    id: Uuid,
    #[diesel(sql_type = SqlUuid)]
    user_id: Uuid,
    #[diesel(sql_type = Binary)]
    secret_hash: Vec<u8>,
    #[diesel(sql_type = Text)]
    access_level: String,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    usable: bool,
}

pub async fn authenticate(
    conn: &mut AsyncPgConnection,
    token: &str,
) -> Result<Option<(Uuid, ApiKeyAccess)>, diesel::result::Error> {
    use diesel::OptionalExtension;
    let Some((prefix, secret)) = token
        .strip_prefix("mk_")
        .and_then(|value| value.split_once('.'))
    else {
        return Ok(None);
    };
    let mut decoded_secret = [0; 32];
    if prefix.len() != 8
        || !prefix
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        || secret.len() != 43
        || BASE64_URL_SAFE_NO_PAD
            .decode_slice(secret, &mut decoded_secret)
            .ok()
            != Some(32)
    {
        return Ok(None);
    }
    let presented_hash = Sha256::digest(secret.as_bytes());
    let row = diesel::sql_query("SELECT k.id, k.user_id, k.secret_hash, k.access_level, (k.expires_at IS NULL OR k.expires_at > now()) AS usable FROM api_keys k JOIN users u ON u.id = k.user_id WHERE k.prefix = $1")
        .bind::<Text, _>(prefix).get_result::<AuthenticationRow>(conn).await.optional()?;
    let stored = row
        .as_ref()
        .map_or([0u8; 32].as_slice(), |row| row.secret_hash.as_slice());
    let matches: bool = presented_hash.as_slice().ct_eq(stored).into();
    let row = match row {
        Some(row) if matches && row.usable => row,
        _ => return Ok(None),
    };
    // The predicate is rechecked after row locking, including across backend replicas.
    diesel::sql_query("UPDATE api_keys SET last_used_at = now() WHERE id = $1 AND (last_used_at IS NULL OR last_used_at <= now() - interval '1 minute')")
        .bind::<SqlUuid, _>(row.id).execute(conn).await?;
    Ok(Some((row.user_id, ApiKeyAccess::parse(&row.access_level))))
}
