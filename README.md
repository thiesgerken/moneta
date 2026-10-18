<h1>
  <img src="frontend/static/brand/lockup.svg#gh-light-mode-only" alt="Moneta" width="296" height="64">
  <img src="frontend/static/brand/lockup-dark.svg#gh-dark-mode-only" alt="Moneta" width="296" height="64">
</h1>

Personal finance tracking: expenses, accounts, balances and categories, shared between several
users. Successor to MoneyDB (Haskell + jQuery, sibling checkout), which it can import from.
Version 3 of the same idea.

- **backend** — Rust: [rocket](https://rocket.rs) + [juniper](https://github.com/graphql-rust/juniper)
  serving a GraphQL API, PostgreSQL 18 for storage (diesel-async)
- **frontend** — SvelteKit 2 / Svelte 5 (runes), [houdini](https://houdinigraphql.com) as the
  GraphQL client, Tailwind 4 + flowbite-svelte for the UI

**Documentation is in [`docs/`](docs/)** — start with [docs/README.md](docs/README.md), which
says what to read in which order. The short version: [concepts.md](docs/concepts.md) for the
data model, [features.md](docs/features.md) for what the app does,
[moneydb.md](docs/moneydb.md) for where the data came from, and
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

You need at least one user before anything is visible, and importing needs every user the
export mentions, `thies` and `kroesus`:

```sh
cargo run -- user --add <name>                     # or, against the compose stack:
docker compose exec backend moneta user --add <name>
```

Importing from MoneyDB then takes the export on stdin. `--clear` wipes everything except the
users:

```sh
cargo run -- import --clear < moneydb-export.json
docker compose exec -T backend moneta import --clear < backend/moneydb-export.json
```

The import is not a row-for-row copy — it folds up the chains MoneyDB needed to express a
prepayment and derives accrual periods the source never recorded.
[docs/moneydb.md](docs/moneydb.md) explains what it does and why.

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
| `MONETA__STOCKDB__URL` | *(empty)* | where the backend reaches stockdb; empty switches the integration off |
| `MONETA__STOCKDB__PUBLIC_URL` | *(empty)* | where a *browser* reaches it, for the link on a depot; falls back to `URL` |
| `MONETA__STOCKDB__TIMEOUT_SECONDS` | `5` | a page load waits on this, so it has to give up quickly |

## CLI

`moneta` is one binary; `serve` is just one of its subcommands.

```sh
moneta serve                         # run the API
moneta user --add|--update|--remove <name> | --list
moneta user --stockdb-token <name>   # paste the token on stdin; empty clears it
moneta account --list x              # the flags take a dummy value, see below
moneta account --sync <id1> <id2> [--invert x]
moneta account --unsync <id>
moneta periodic --list               # rules and how many expenses each is due for
moneta periodic --run                # generate the expenses that are due
moneta import [--clear] < moneydb-export.json
moneta schema                        # the GraphQL schema as SDL, for frontend/schema.graphql
```

`import` matches users by name, so create them with `user --add` before importing; `--clear`
wipes everything except users.

Note the odd argument shapes on `account`: `--list` and `--invert` are declared as value-taking
options rather than flags, so they need a throwaway argument (`moneta account --list x`). Worth
fixing, hasn't been.

## Tests and CI

```sh
cd backend  && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --all -- --check
cd backend  && PGURL=postgres://moneta:moneta@localhost:5433/moneta tests/run-sql-tests.sh
cd frontend && pnpm build && pnpm check && pnpm lint && pnpm test
```

`tests/run-sql-tests.sh` applies every migration to an empty database and then asserts that the
rendered views, the CHECK constraints and the analysis functions do what they claim — the views
carry the money maths, so they are where a mistake would be expensive. `pnpm test` is vitest
over the pure modules under `src/lib`; there are no component or end-to-end tests.

CI (`.gitea/workflows/ci.yml`) runs exactly those, then builds and pushes both images on pushes
to `main`, and then restarts the two deployments in the cluster. A merge to `main` is a deploy.

## Status

Working: auth, CRUD for all four entities, the rendering pipeline, expense changelog with
diffs, computed balances, MoneyDB import, periodic and automation rules (both imported from
MoneyDB, generation runnable from the CLI or the API), and the analysis dashboard with its
value-date/accrual toggle.

Missing: an upload client feeding `deliveries.submit` from a bank export, receipts in the UI,
and push notifications. See `TODO.md`.
