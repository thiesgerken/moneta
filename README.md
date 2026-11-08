<h1>
  <img src="frontend/static/brand/lockup.svg#gh-light-mode-only" alt="Moneta" width="296" height="64">
  <img src="frontend/static/brand/lockup-dark.svg#gh-dark-mode-only" alt="Moneta" width="296" height="64">
</h1>

Personal finance tracking: expenses, accounts, balances and categories, shared between several
users.

- **backend** — Rust: [rocket](https://rocket.rs) + [juniper](https://github.com/graphql-rust/juniper)
  serving a GraphQL API, PostgreSQL 18 for storage (diesel-async)
- **frontend** — SvelteKit 3 / Svelte 5 (runes), [houdini](https://houdinigraphql.com) as the
  GraphQL client, Tailwind 4 + flowbite-svelte for the UI

**Documentation is in [`docs/`](docs/)** — start with [docs/README.md](docs/README.md), which
says what to read in which order. The short version: [concepts.md](docs/concepts.md) for the
data model, [features.md](docs/features.md) for what the app does, and
[backend.md](docs/backend.md) / [frontend.md](docs/frontend.md) for the code. If you are about
to change something, [AGENTS.md](AGENTS.md) is the list of traps already stepped in.

## Running it locally

```sh
docker compose up -d postgres               # postgres on 127.0.0.1:5433
cd backend  && cargo run -- serve           # API + graphiql on :8181
cd frontend && pnpm install && pnpm dev     # UI on :5173, proxies /graphql to :8181
```

The full stack (traefik in front of backend, frontend and postgres) comes up with
`docker compose up -d`, reachable on <http://localhost:8485>. It publishes the backend on 8181
as well, so `pnpm dev` talks to it without any change — run one backend or the other, not both.

**PostgreSQL 18 is a hard requirement**: ids default to `uuidv7()`, which does not exist
before it.

You need at least one user before anything is visible:

```sh
cargo run -- user --add <name>                     # or, against the compose stack:
docker compose exec backend moneta user --add <name>
```

### Configuration

Settings come from environment variables prefixed `MONETA__`, nested with `__`
(`MONETA__DATABASE__URL`), or a `.env` file next to the backend. Defaults live in
`backend/src/config.rs`.

| Variable | Default | Notes |
| --- | --- | --- |
| `MONETA__DATABASE__URL` | `postgres://moneta:moneta@localhost:5433/moneta` | matches the port docker-compose exposes |
| `MONETA__API__PORT` | `8181` | |
| `MONETA__API__ADDRESS` | `127.0.0.1` | |
| `MONETA__API__SECRET_KEY` | *(empty)* | signs the session cookie; `openssl rand -base64 32` |
| `MONETA__RENDITIER__URL` | *(empty)* | where the backend reaches renditier; empty switches the integration off |
| `MONETA__RENDITIER__PUBLIC_URL` | *(empty)* | where a *browser* reaches it, for the link on a depot; falls back to `URL` |
| `MONETA__RENDITIER__TIMEOUT_SECONDS` | `5` | a page load waits on this, so it has to give up quickly |

## Machine access and Ferret

Sign in with your browser, open **API-Schlüssel**, and create a named **Nur lesen** key.
Read-only is sufficient for Ferret. Copy the full `mk_...` token from the one-time dialog
into your client's secret storage. The optional expiry defaults to no expiry.

Use the existing GraphQL endpoint, not a separate REST API:

```sh
read -rs MONETA_API_KEY
export MONETA_API_KEY
curl --silent --show-error 'http://localhost:8181/graphql' \
  -H "Authorization: Bearer $MONETA_API_KEY" \
  -H 'Content-Type: application/json' \
  --data '{"query":"{ expenses { list(limit: 10) { edges { node { id title effectiveAmount } } } } }"}'
unset MONETA_API_KEY
```

Treat the key like a password. Avoid shell tracing, request-header logging and pasting keys
into issues. Moneta stores only a hash of the random secret plus a nonsecret lookup prefix.
There is no token recovery. Replace a lost key by creating another and removing the old one.
Password changes do **not** invalidate API keys.

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

Remove a key in the browser and confirm to revoke it. Removal physically deletes the row
and removes it from the list; it fails on the next request. Expired keys remain listed as
**Abgelaufen** until removed. Last use is bounded to one update per minute and counts
successful authentication, even if the later operation fails. It is usage metadata, not an
audit trail. Expiry, deletion and the owner's continued existence are checked on every request.

## CLI

`moneta` is one binary; `serve` is just one of its subcommands.

```sh
moneta serve                         # run the API
moneta user --add|--update|--remove <name> | --list
moneta user --renditier-token <name>   # paste the token on stdin; empty clears it
moneta account --list x              # the flags take a dummy value, see below
moneta account --sync <id1> <id2> [--invert x]
moneta account --unsync <id>
moneta periodic --list               # rules and how many expenses each is due for
moneta periodic --run                # generate the expenses that are due
moneta schema                        # the GraphQL schema as SDL, for frontend/schema.graphql
```

Note the odd argument shapes on `account`: `--list` and `--invert` are declared as value-taking
options rather than flags, so they need a throwaway argument (`moneta account --list x`). Worth
fixing, hasn't been.

## Tests and CI

```sh
cd backend  && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --all -- --check
cd backend  && PGURL=postgres://moneta:moneta@localhost:5499/moneta tests/run-sql-tests.sh
cd frontend && pnpm build && pnpm check && pnpm lint && pnpm test
```

`tests/run-sql-tests.sh` applies every migration to an empty **scratch PostgreSQL 18 database**
and then asserts that the rendered views, the CHECK constraints and the analysis functions do
what they claim. Never point it at a database holding real data. The views carry the money
maths, so they are where a mistake would be expensive. `pnpm test` is vitest over the pure
modules under `src/lib`; there are no component or end-to-end tests.

CI (`.gitea/workflows/ci.yml`) runs exactly those, then builds and pushes both images on pushes
to `main`, and then restarts the two deployments in the cluster. A merge to `main` is a deploy.

## Status

Working: auth, CRUD for all four entities, the rendering pipeline, expense changelog with
diffs, computed balances, periodic and automation rules (generation runnable from the CLI or
the API), and the analysis dashboard with its value-date/accrual toggle.

Missing: an upload client feeding `deliveries.submit` from a bank export, receipts in the UI,
and push notifications. See `TODO.md`.
