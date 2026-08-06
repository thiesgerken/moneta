# Moneta

Personal finance tracking: expenses, accounts, balances and categories, shared between
several users. Successor to MoneyDB (Haskell + jQuery, sibling checkout), which it can
import from. Version 3 of the same idea.

- **backend** — Rust: [rocket](https://rocket.rs) + [juniper](https://github.com/graphql-rust/juniper)
  serving a GraphQL API, PostgreSQL 18 for storage (diesel-async)
- **frontend** — SvelteKit 2 / Svelte 5 (runes), [houdini](https://houdinigraphql.com) as the
  GraphQL client, Tailwind 4 + flowbite-svelte for the UI

## Running it locally

```sh
docker compose up -d postgres               # postgres on 127.0.0.1:5433
cd backend  && cargo run -- serve           # API + graphiql on :8181
cd frontend && pnpm install && pnpm dev     # UI on :5173, proxies /graphql to :8181
```

The full stack (traefik in front of backend, frontend and postgres) comes up with
`docker compose up -d`, reachable on <http://localhost:8485>. It publishes the backend on
8181 as well, so `pnpm dev` talks to it without any change — run one backend or the other,
not both.

You need at least one user before anything is visible, and importing needs every user the
export mentions, `thies` and `kroesus`:

```sh
cargo run -- user --add <name>                     # or, against the compose stack:
docker compose exec backend moneta user --add <name>
```

Importing from MoneyDB then takes the export on stdin. `--clear` wipes everything except
the users:

```sh
cargo run -- import --clear < moneydb-export.json
docker compose exec -T backend moneta import --clear < backend/moneydb-export.json
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

## CLI

`moneta` is one binary; `serve` is just one of its subcommands.

```sh
moneta serve                         # run the API
moneta user --add|--update|--remove <name> | --list
moneta account --list x              # the flags take a dummy value, see below
moneta account --sync <id1> <id2> [--invert x]
moneta account --unsync <id>
moneta periodic --list               # rules and how many expenses each is due for
moneta periodic --run                # generate the expenses that are due
moneta import [--clear] < moneydb-export.json
```

`import` matches users by name, so create them with `user --add` before importing;
`--clear` wipes everything except users.

Note the odd argument shapes on `account`: `--list`, `--rerender` and `--invert` are declared
as value-taking options rather than flags, so they need a throwaway argument
(`moneta account --list x`). Worth fixing, hasn't been.

## Data model

Two layers, and understanding the split is the key to the whole backend.

**Upstream collections** — `expenses`, `balances`, `accounts`, `categories`, `users`,
`expense_changes`. These are the truth. An expense is owned by nobody in particular: it has
a list of *transactions*, each pointing at an account, and each account has an owner.

**Rendered collections** — `expenses_rendered`, `balances_rendered`. One document *per user
per upstream entry*, precomputed by `src/rendering.rs`. Every read path (GraphQL queries,
analysis) hits these, never the upstream ones. They exist because "what did this expense
cost *me*" is expensive to answer on the fly and is asked constantly.

Rendering resolves three things:

- **ownership fraction** — a jointly held account contributes only its owner's share to
  `effectiveAmount`
- **fractional transactions** — a transaction may carry a `fraction` instead of an `amount`,
  resolved against the expense's `referenceAmount`
- **account synchronization** — two accounts belonging to different users can be linked, so
  one person's entry shows up in the other's view too. With `invert`, the sign flips, which
  is how debts between the two users are modelled

**Any write to an upstream document must re-render.** The mutations do this; if you touch
rendering logic, `moneta account --rerender x` rebuilds everything from scratch.

Money is stored as integer cents (`amount: i32`). Computed values that involve fractions are
`f64` (`calculatedAmount`, `effectiveAmount`).

### Rules

Two kinds of rule create expenses by copying a *template* expense (`periodic_rules`,
`automation_rules`, logic in `src/rules.rs`):

- **periodic rules** fire on a schedule (daily/weekly/monthly/yearly). Occurrences are
  anchored on the template's `bookingStart`, so a monthly rule keeps its day of month however
  long it lay dormant. `titleReplacement` substitutes `%d`/`%m`/`%y`, `amountReplacement`
  replaces the reference amount and, when the template has exactly one fixed transaction,
  that transaction too — the shape almost every shared expense has.
- **automation rules** match a delivered bank transaction on any of statement regex, date
  regex, exact amount and account; the highest `priority` match wins and its template is
  copied with the delivered data imposed on it.

Run periodic generation with `moneta periodic --run`, the `periodicRules.run` mutation, or
the button on the rules page. In production this is a scheduled job (k8s CronJob); the
compose stack runs the same command from a `periodic` sidecar so both paths stay identical.
Deliberately not an in-process timer: it would fire once per backend replica and hide its
failures in the server log. It is idempotent — each rule's `lastCreation` only advances
after its expenses are stored.

Note that a template need not be flagged `isTemplate`: moneydb let ordinary past expenses
double as prototypes, and the import preserves that rather than rewriting history.

Bank statement lines are handed in through the `deliveries.submit` mutation, which
deduplicates, matches, and creates the resulting expenses. Each line comes back as `CREATED`,
`DUPLICATE`, `UNMATCHED` or `FAILED`.

Statements carry no stable per-line id, so a delivery counts as already seen when the same
amount hits the same account within 24 hours. The statement text is deliberately not part of
that comparison — banks reword it between a provisional and a final booking of the same
payment. The consequence is that a genuine second identical payment on the same day is
treated as a duplicate, which is the safer way round to be wrong.

Every line is remembered whether or not a rule matched, so re-uploading an overlapping
statement is cheap and safe. `FAILED` lines are the exception: they are not remembered, so
fixing the rule and uploading again picks them up.

## Backend layout

```
src/
  bin/moneta.rs     entry point, config loading, subcommand dispatch
  cli/              user, account and import subcommands
  config.rs         settings struct
  lib.rs            password hashing, connection pool, migration runner
  migrations/       diesel migrations; the schema and the rendered views live here
  schema.rs         generated by `diesel print-schema`
  schema_views.rs   hand-written table! definitions for the views (they have no primary key
                    for print-schema to infer)
  models/           row structs (+ models/rendering.rs for what the views return)
  rendering.rs      what is left of the money math once the views own it
  rules.rs          periodic and automation rules; pure parts well covered by tests
  queries.rs        shared queries, incl. loading and writing the expense aggregate
  graphql/          one module per domain, root.rs wires the schema and auth
  import.rs         MoneyDB JSON import
```

An expense spans three tables (`expenses`, `expense_transactions`,
`expense_categorizations`), so every write goes through `queries::insert_expense` or
`replace_expense` inside a transaction, together with its changelog entry.

Authentication is a signed private cookie holding the user id; `require_user` in
`graphql/root.rs` gates every resolver. Per-user scoping is the resolver's own
`user_id` filter against the rendered views — callers have no way to express one.

## Frontend layout

Seven pages (`expenses`, `balances`, `categories`, `accounts`, `rules`, `deliveries`,
`login`) plus a dashboard that currently only shows the changelog. Each page is a thin route
wrapping a `*Table.svelte` component in `src/lib/`, paired with a `*Modal.svelte` for
create/edit.

`rules` holds both rule kinds in tabs; `deliveries` lists what has been handed in, with a
filter for the lines no rule claimed — that list is the queue of rules still to write.

Queries are declared inline with `graphql(...)` and fetched explicitly:

```svelte
const store = graphql(`query Foo { ... }`);
$effect(() => void store.fetch());
```

For the filtered tables the variables are computed from component state and passed through,
which is what makes the effect re-run when a filter changes:

```svelte
$effect(() => void expenseStore.fetch({ variables: expenseVariables() }));
```

Filtering and sorting are done **server-side** through typed arguments: each list query takes
a `search` string, whatever filters that entity has, and a `sortBy` enum plus `sortDir`. The
components used to build MongoDB query documents in the browser and send them as strings;
nothing on the wire is a query language any more.

`schema.graphql` is checked in and is what codegen runs against. Refresh it with the backend
running:

```sh
cd frontend && pnpm exec houdini pull-schema
```

## Tests and CI

```sh
cd backend  && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --all -- --check
cd backend  && PGURL=postgres://moneta:moneta@localhost:5433/moneta tests/run-sql-tests.sh
cd frontend && pnpm build && pnpm check && pnpm lint
```

`tests/run-sql-tests.sh` applies every migration to an empty database and then asserts that
the rendered views and the CHECK constraints do what they claim — the views carry the money
math, so they are where a mistake would be expensive.

CI (`.gitea/workflows/ci.yml`) runs exactly those, then builds and pushes both images on
pushes to `main`/`devel`. There are no frontend tests.

## Status

Working: auth, CRUD for all four entities, the rendering pipeline, expense changelog with
diffs, computed balances, MoneyDB import.

Also working: periodic and automation rules, both imported from MoneyDB, with periodic
generation runnable from the CLI or the API.

Missing: the analysis dashboard (MoneyDB had calendar, category sunburst, history, monthly
overviews, prepayments, statements), receipts in the UI, and push notifications. See
`TODO.md`.
