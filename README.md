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
cd frontend && npm install && npm run dev   # UI on :5173, proxies /graphql to :8181
```

The full stack (traefik in front of backend, frontend and postgres) comes up with
`docker compose up -d`, reachable on <http://localhost:8485>. It publishes the backend on 8181
as well, so `npm run dev` talks to it without any change — run one backend or the other, not both.
Both images are built with Nix, not from Dockerfiles: compose pulls them from the registry, or on
x86_64-linux `nix build .#backend-image && ./result | docker load` (and the same for
`frontend-image`) builds them locally under the same names.

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
| `MONETA__API__TRUSTED_HEADER_AUTHENTICATION` | `false` | sign people in by the `Remote-User` header a login proxy sets, see below |
| `MONETA__API__TRUSTED_PROXIES` | *(empty)* | CIDRs, comma-separated, that `Remote-User` is believed from; required when the above is on |

### Signing in through a login proxy

With `MONETA__API__TRUSTED_HEADER_AUTHENTICATION=true`, a reverse proxy that does the login
(Authelia, oauth2-proxy, traefik's forward auth, ...) can hand over who is calling in a
`Remote-User` header carrying an existing moneta user name. Nobody is created for a name moneta
does not know, and such a user needs no password, so an imported user works straight away.

The header is believed only when the TCP connection comes from an address in
`MONETA__API__TRUSTED_PROXIES` (e.g. `10.42.0.0/16`, the pod network the proxy runs in).
`X-Forwarded-For` and `X-Real-IP` play no part in that decision. Switching the header on with an
empty list is a startup error. From anywhere else the header is ignored and logged.

This is safe only when the backend is reachable **solely through that proxy**, and when the
proxy **removes a `Remote-User` the client sent** before setting its own. Anything that can open
a connection from inside the allowlist can sign in as anybody. An API key still wins over the
header, and the header wins over a session cookie. Signing out is the proxy's business: the
app's own logout cannot end a session it does not keep.

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
moneta export > moneta-export.json   # all users' data as JSON, see below
moneta import < moneta-export.json   # only into a database without user data
moneta schema                        # the GraphQL schema as SDL, for frontend/schema.graphql
```

Note the odd argument shapes on `account`: `--list` and `--invert` are declared as value-taking
options rather than flags, so they need a throwaway argument (`moneta account --list x`). Worth
fixing, hasn't been.

### Export and import

`moneta export` writes the whole database's user data to stdout as JSON: users, categories,
accounts, expenses with their transactions, balances, periodic and automation rules, seen
deliveries and the expense changelog. Credentials stay out: no password hashes, API keys or
renditier tokens. Sessions are cookies and were never in the database. Ids are kept, so the
changelog still points at what it describes.

`moneta import` reads such a file back, and only into a database that holds no user data yet
(the migrations having run is fine; a single user is not). Anything else is refused before the
first write: an expense books across several users' accounts, so there is no replacing one
user's data without breaking what they share with the others. The file is checked for
duplicate ids and dangling references first, and whatever the database rejects after that
rolls the whole import back. Imported users have no password; give each one with
`moneta user --update <name>`, and set renditier tokens again with `--renditier-token`.

The file holds everything personal there is, so it goes nowhere but where you redirect it:
both commands are CLI-only, nothing is served over HTTP, and `*-export.json` is gitignored.
The export complements a `pg_dump` rather than replacing it: it is readable and independent
of the schema's internals, but it leaves the credentials behind and restores only into an empty
database.

## Tests and CI

```sh
cd backend  && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --all -- --check
cd backend  && PGURL=postgres://moneta:moneta@localhost:5499/moneta tests/run-sql-tests.sh
cd frontend && npm run build && npm run check && npm run lint && npm test
```

`tests/run-sql-tests.sh` applies every migration to an empty **scratch PostgreSQL 18 database**
and then asserts that the rendered views, the CHECK constraints and the analysis functions do
what they claim. Never point it at a database holding real data. The views carry the money
maths, so they are where a mistake would be expensive. `npm test` is vitest over the pure
modules under `src/lib`; there are no component or end-to-end tests.

CI (`.gitea/workflows/ci.yml`) runs exactly those inside the Nix dev shell, so it tests with
the toolchain a `nix develop` gives locally. On pushes to `main` it then builds and pushes both
images (with Nix, from the same toolchain) and restarts the two deployments in the cluster. A
merge to `main` is a deploy.

## Status

Working: auth, CRUD for all four entities, the rendering pipeline, expense changelog with
diffs, computed balances, periodic and automation rules (generation runnable from the CLI or
the API), and the analysis dashboard with its value-date/accrual toggle.

Missing: an upload client feeding `deliveries.submit` from a bank export, receipts in the UI,
and push notifications. See `TODO.md`.

## License

MIT. See [LICENSE](LICENSE).
