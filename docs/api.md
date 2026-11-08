# API keys

Moneta has one API, the GraphQL endpoint the frontend uses; there is no separate REST API. A
browser signs in with a session cookie or through a login proxy (see the
[README](../README.md#-signing-in)); scripts and agents such as Ferret use an API key.

The backend serves GraphiQL at its root (<http://localhost:8181/> when run locally), which is
the quickest way to try a query. The schema is checked in and documented in place at
[`frontend/schema.graphql`](../frontend/schema.graphql); [backend.md](backend.md#graphql-conventions)
has the conventions behind it and the details of how keys are stored and checked.

## Creating one

Sign in with your browser, open **API-Schlüssel**, and create a named **Nur lesen** key.
Read-only is sufficient for Ferret. Copy the full `mk_...` token from the one-time dialog
into your client's secret storage. The optional expiry defaults to no expiry.

Send it as a bearer token:

```sh
read -rs MONETA_API_KEY
export MONETA_API_KEY
curl --silent --show-error 'http://localhost:8181/graphql' \
  -H "Authorization: Bearer $MONETA_API_KEY" \
  -H 'Content-Type: application/json' \
  --data '{"query":"{ expenses { list(limit: 10) { edges { node { id title effectiveAmount } } } } }"}'
unset MONETA_API_KEY
```

## Handling it

Treat the key like a password. Avoid shell tracing, request-header logging and pasting keys
into issues. Moneta stores only a hash of the random secret plus a nonsecret lookup prefix.
There is no token recovery. Replace a lost key by creating another and removing the old one.
Password changes do **not** invalidate API keys.

## What a key may do

**Nur lesen** permits queries only. **Lesen und schreiben** adds domain mutations without
widening your browser account's permissions. Neither level may list/create/remove API keys,
log in/out, change identity or record a human visit. For example, a write key can create a
balance on one of its owner's accounts:

```sh
read -rs MONETA_API_KEY
export MONETA_API_KEY
curl --silent --show-error 'http://localhost:8181/graphql' \
  -H "Authorization: Bearer $MONETA_API_KEY" \
  -H 'Content-Type: application/json' \
  --data '{"query":"mutation($balance: BalanceInput!) { balances { create(balance: $balance) { id } } }","variables":{"balance":{"accountId":"YOUR-ACCOUNT-UUID","date":"2026-10-18T12:00:00Z","amount":10000,"comments":"Recorded balance"}}}'
unset MONETA_API_KEY
```

An explicit Authorization header always takes precedence over cookies, even if it is invalid.
Bearer requests never receive session-cookie changes. Invalid credentials return GraphQL
`UNAUTHENTICATED`; forbidden credential operations return `FORBIDDEN`. Read-only rejection
happens before mutation side effects, including aliases, fragments, selected operations and
batch requests. A write operation may commit permitted fields before a later session-only
field fails; requests are not transactions.

## Revoking, expiry and last use

Remove a key in the browser and confirm to revoke it. Removal physically deletes the row
and removes it from the list; it fails on the next request. Expired keys remain listed as
**Abgelaufen** until removed. Last use is bounded to one update per minute and counts
successful authentication, even if the later operation fails. It is usage metadata, not an
audit trail. Expiry, deletion and the owner's continued existence are checked on every request.
