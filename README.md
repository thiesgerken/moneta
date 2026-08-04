# Moneta

Personal finance tracking: expenses, accounts, balances and categories, shared between
several users. Successor to MoneyDB (Haskell + jQuery, sibling checkout), which it can
import from. Version 3 of the same idea.

- **backend** — Rust: [rocket](https://rocket.rs) + [juniper](https://github.com/graphql-rust/juniper)
  serving a GraphQL API, MongoDB for storage
- **frontend** — SvelteKit 2 / Svelte 5 (runes), [houdini](https://houdinigraphql.com) as the
  GraphQL client, Tailwind 4 + flowbite-svelte for the UI

## Running it locally

```sh
docker compose up -d mongo                  # mongo on 127.0.0.1:27018
cd backend  && cargo run -- serve           # API + graphiql on :8181
cd frontend && pnpm install && pnpm dev     # UI on :5173, proxies /graphql to :8181
```

You need at least one user before anything is visible:

```sh
cd backend && cargo run -- user --add <name>
```

The full stack (traefik in front of backend, frontend and mongo) comes up with
`docker compose up -d`, reachable on <http://localhost:8485>.

### Configuration

Settings come from environment variables prefixed `MONETA__`, nested with `__`
(`MONETA__DATABASE__URL`), or a `.env` file next to the backend. Defaults live in
`backend/src/config.rs`.

| Variable | Default | Notes |
| --- | --- | --- |
| `MONETA__DATABASE__URL` | `mongodb://localhost:27018` | matches the port docker-compose exposes |
| `MONETA__DATABASE__NAME` | `moneta` | |
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
moneta account --rerender x          # rebuild every rendered document
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

Run periodic generation with `moneta periodic --run` (a cron job in production) or the
`periodicRules.run` mutation. It is idempotent — each rule's `lastCreation` only advances
after its expenses are stored.

Note that a template need not be flagged `isTemplate`: moneydb let ordinary past expenses
double as prototypes, and the import preserves that rather than rewriting history.

**The delivery endpoint that feeds automation rules does not exist yet.** The matching and
copying logic is implemented and tested, but nothing calls it until there is an API to
accept bank statement lines.

## Backend layout

```
src/
  bin/moneta.rs     entry point, config loading, subcommand dispatch
  cli/              user, account and import subcommands
  config.rs         settings struct
  lib.rs            password hashing, db connection, index creation
  models/           upstream documents (+ models/rendering.rs for the rendered ones)
  rendering.rs      the money math; pure functions, well covered by tests
  rules.rs          periodic and automation rules; pure parts well covered by tests
  queries.rs        shared mongo helpers, incl. the datetime serde adapters
  graphql/          one module per domain, root.rs wires the schema and auth
  import.rs         MoneyDB JSON import
```

Authentication is a signed private cookie holding the user id; `require_user` in
`graphql/root.rs` gates every resolver. Per-user scoping happens in
`graphql/util.rs::merge_filter`, which applies a `userId` override *after* the caller's own
filter so it cannot be widened — see the tests there before changing it.

## Frontend layout

Five pages (`expenses`, `balances`, `categories`, `accounts`, `login`) plus a dashboard that
currently only shows the changelog. Each page is a thin route wrapping a `*Table.svelte`
component in `src/lib/`, paired with a `*Modal.svelte` for create/edit.

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

Filtering and sorting are done **server-side**: the components build a MongoDB query document
as JSON and send it in the `filter`/`sort` string arguments. That is why the backend parses
caller-supplied filters at all.

`schema.graphql` is checked in and is what codegen runs against. Refresh it with the backend
running:

```sh
cd frontend && pnpm exec houdini pull-schema
```

## Tests and CI

```sh
cd backend  && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --all -- --check
cd frontend && pnpm build && pnpm check && pnpm lint
```

CI (`.gitea/workflows/ci.yml`) runs exactly those, then builds and pushes both images on
pushes to `main`/`devel`. There are no frontend tests and no integration tests against a real
mongo — the backend tests cover the rendering math and the filter merging, which is where the
bugs would actually hurt.

## Status

Working: auth, CRUD for all four entities, the rendering pipeline, expense changelog with
diffs, computed balances, MoneyDB import.

Also working: periodic and automation rules, both imported from MoneyDB, with periodic
generation runnable from the CLI or the API.

Missing: the analysis dashboard (MoneyDB had calendar, category sunburst, history, monthly
overviews, prepayments, statements), the delivery endpoint that would let automation rules
actually fire, receipts in the UI, push notifications, and any frontend for rules at all.
See `TODO.md`.
