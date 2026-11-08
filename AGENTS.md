# Working on this repo

Read [`docs/`](docs/) first for what the project is: [`docs/concepts.md`](docs/concepts.md) is
the two-layer data model and the three dates, [`docs/features.md`](docs/features.md) is what
the application does, and [`docs/backend.md`](docs/backend.md) /
[`docs/frontend.md`](docs/frontend.md) are the code.
`README.md` is how to run it.

This file is the operational stuff: what to run to verify, and the traps that have already been
stepped in. `TODO.md` is what is deliberately not built yet.

With Nix, the dev shell (`direnv allow` or `nix develop`) brings the pinned Rust toolchain, node
(with npm), cargo-llvm-cov and PostgreSQL 18. CI runs every check inside it, and both images are
built with the same toolchain, so `rust-toolchain.toml` and `flake.lock` are the one place the
versions live. Locally, Postgres for the tests still runs in Docker, as below.

## Verify your changes

```sh
cd backend  && cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test
cd backend  && PGURL=postgres://moneta:moneta@localhost:5499/moneta tests/run-sql-tests.sh
cd frontend && npm run build && npm run check && npm run lint && npm test
```

The SQL tests need an empty PostgreSQL 18 database of their own. They apply every migration,
without recording Diesel migration state, so use a dedicated scratch database that no backend
will connect to. Do not point them at a database holding real data.

The DB-backed Rust suites take `TEST_DATABASE_URL`: run `cargo test --test scoping` and then
`cargo test --test api_keys` against another disposable database. They migrate and truncate
fixtures, so never run both concurrently against one database. The API-key suite exercises the
production GraphQL HTTP handler with dummy users; `MONETA_REQUIRE_DB_TESTS=1` forbids skipping
it when the URL is absent. CI runs both sequentially in its database-backed job.

CI runs exactly these and is strict — `clippy -D warnings` and `fmt --check` both fail the
build, including on test code.

On a pull request it also comments the coverage of both halves, editing the same comment on
every push. It is a report, not a gate: nothing fails on a number going down. Locally the
same figures come from `cargo llvm-cov --summary-only` and `npm test -- --coverage`.

A push to `main` also builds both images and restarts the deployments in the cluster, so a
merge is a deploy. `main` is the only branch CI runs on; work happens in pull requests
against it, which run the tests and build both images but neither push nor deploy.

`npm test` is vitest over the plain modules under `src/lib` — the pure ones, where the logic
is worth pinning down and no browser is needed. It has a `vitest.config.ts` of its own so it
loads neither houdini's plugin, whose watcher held every run open for ten seconds after the
last assertion, nor anything else the app needs only to build.

**Run `npm run build` before `npm run check` or `npm run lint`.** The build triggers houdini codegen;
without it `$houdini` types are stale or missing and you will chase type errors that aren't
real. If codegen gets into a bad state, `rm -rf .houdini .svelte-kit` and rebuild.

## Never touch `volumes/postgres` directly

That directory is real personal financial data. It is gitignored and there is no other copy
of most of it.

Use the unit tests and the SQL fixtures against a dedicated throwaway PostgreSQL 18 container:

```sh
docker run -d --name moneta-sql-tests -e POSTGRES_USER=moneta -e POSTGRES_PASSWORD=moneta \
    -e POSTGRES_DB=moneta -p 127.0.0.1:5499:5432 postgres:18-alpine
cd backend
cargo test
PGURL=postgres://moneta:moneta@localhost:5499/moneta tests/run-sql-tests.sh
```

The SQL files create their own test rows inside transactions that roll back. This workflow
does not need a financial export or access to the real database. Migration compatibility with
existing data uses a restored database in a separate throwaway container, as described below.

## The cluster runs the real thing, and its database is reachable

`moneta.gerken.haus` is a live deployment in k3s, namespace `apps-moneta`, context
`gerken-haus`. A merge to `main` deploys to it. Its postgres answers:

```sh
kubectl -n apps-moneta exec deploy/postgres -- psql -U moneta -d moneta -c 'select ...'
```

**That is the same personal financial data as `volumes/postgres`, and there is no other copy.**
Reading it is fine and is often the fastest way to settle a question about real behaviour — do
that rather than guess. Writing to it is not: any `UPDATE`, `DELETE`, `TRUNCATE` or schema change
applied by hand needs Thies to allow that specific change, each time it comes up. Permission
for one write is not permission for the next. Hand him the command to run instead; that is the
normal way this goes, not a fallback.

A nightly CronJob `moneta-export` dumps it at 04:00 (`pg_dump --format=custom`) onto the
`moneta-export` PVC, keeping one complete dump. That is the rollback, and it is up to 24 hours
old — force a fresh one before anything irreversible:

```sh
kubectl -n apps-moneta create job --from=cronjob/moneta-export precutover
```

## The data model carries data now

The database is the source of truth and cannot be rebuilt from an old export. Deliveries,
edits, confirmations and renditier links must survive every change, together with the existing
UUIDs and historical changelog snapshots.

A migration has to carry the rows across rather than assume they can be rebuilt. Dropping a
column, tightening a constraint or renaming an enum value needs the `up.sql` that moves the data
with it, and it runs unattended against the real database on the next merge to `main`. Where
that cannot be done in one step, plan two releases and say so, rather than guessing that the
table is empty enough to get away with it.

Test a migration against real data before merging it — restore the nightly dump into a
throwaway container and let the binary migrate it:

```sh
docker run -d --name restore-test -e POSTGRES_USER=moneta -e POSTGRES_PASSWORD=moneta \
    -e POSTGRES_DB=moneta -p 127.0.0.1:5497:5432 postgres:18-alpine
docker exec -i restore-test pg_restore --clean --if-exists --no-owner --no-privileges \
    -U moneta -d moneta < moneta.dump
MONETA__DATABASE__URL=postgres://moneta:moneta@localhost:5497/moneta cargo run -- account --list all
```

Migrations run before every subcommand except `schema`, so any command applies them.

**Postgres 18 is a hard requirement**: the tables default their ids with `uuidv7()`, which
does not exist before 18.

## Traps

**Delivery dedup never looks at the expenses that already exist.** `is_duplicate` compares a
new line against `seen_deliveries` and nothing else — account, amount, date within 24 hours,
statement text deliberately ignored. So anything that puts bank lines into the database by
another route has to record them in `seen_deliveries`, or the next statement upload delivers
them all again.

**A stored date is an instant, and calendar arithmetic on it needs the frame it was written
in.** The editor writes midnight where the reader sits: 23:00 UTC the day before in winter,
22:00 in summer. Stored dates can also be midday UTC. Read the day off such an instant in UTC
and a period entered as starting on the 1st begins on the 31st of the month before, which threw
a monthly rule's grid off by a day and made a span across the clock change stop counting as
whole months. `accrual::on` converts to `accrual::ZONE` before anything reads a day or a month;
`accrual::day` builds every grid point at midday UTC, which is the same date in either frame.
Keep that split — reading names the zone, writing does not need to.

**The date axis stops where it stops.** Balances, the statement and every per-account series
stay on the value date on purpose, and the Gesamt wealth curve is the one exception — see
[docs/analysis-dashboard.md](docs/analysis-dashboard.md#the-date-axis) before extending the
toggle anywhere.

**Money is integer cents.** `amount: i32` everywhere upstream. Only fraction-derived values
(`calculatedAmount`, `effectiveAmount`) are `f64`. Do not introduce floats into stored
amounts.

**Rendering is a view, not stored state.** `expenses_rendered`, `expense_transactions_rendered`
and `balances_rendered` derive the per-user view from the upstream tables, so there is nothing
to keep in sync and no rerender step. If you change them, re-run `tests/sql/rendered_views.sql`:
it mutation-tests the ownership-fraction and sync-inversion paths, and four of its cases
passed against an earlier, wrong version of the views.

**Never hold a pool connection while awaiting a dataloader.** A loader batch takes a connection
of its own, so a resolver that keeps one and asks for a second deadlocks the moment there are as
many concurrent requests as the pool has connections: each holds one, each waits for another,
and the pool can never refill. It does not recover. From outside it looks like the backend
accepting requests and answering none — the log stops at `Matched` with no `Outcome`, and
postgres shows its connections idle with a stale `last query`. `compute()` and `overviews()` both
did this; both now scope the connection into a block that ends before the loader runs.
`prime_many` is safe, it touches no connection.

**The pool is 16 with a 15 second wait timeout, and both numbers are deliberate.** After the
above, a request holds one connection at a time, so the size is just a cap on concurrent
DB-touching requests — a page load fires around six. Bigger buys nothing: the pod has a 1 CPU
limit and postgres is happier with a small pool than a large one. `max_connections` is 100, so
`16 × replicas` plus the cronjobs is the budget to respect before scaling out. The timeout is
what makes a mistake survivable: without it the wait is unbounded and one bad resolver takes the
process down silently. `Timeout occurred while waiting for a slot to become available` in the
log is the signal that the pool is genuinely too small — or that something is nesting again.
Size it off that line, not off a guess.

**A new mutation has to invalidate the analysis cache.** `analysis.balances`, `history` and
`overviews` are memoized in process for ten minutes (`graphql/cache.rs`). What keeps that
honest is the mutation root in `root.rs`: financial branches set `context.wrote`, and
`graphql_handler` empties the cache once the request is done. Add a financial mutation branch
without that line and a write leaves stale months on the screen until the TTL runs out.
`users` and `apiKeys` deliberately do not: visit markers, credential management and
authentication/last-use writes do not affect financial answers. `root.rs` and the real-handler
API-key tests hold those exceptions.

**Keys are rounded to ten minutes, on purpose.** Every range the frontend asks for ends at
`Date.now()`, so a key built from the exact arguments is a fresh one on every page load and
the cache never hits. `cache::bucket` floors the dates, which means an entry can answer a
request for a window ending slightly after the one it was computed for. Anything that starts
caching a query where that is *not* invisible needs its own key, not this one.

**Render the transactions once per request, not once per row you need them for.**
`expense_users` is a `UNION` over the whole transaction table, so no filter on account or date
narrows it — reading `expense_transactions_rendered` from inside a lateral rebuilds the entire
rendering on every invocation. `simulate_balances` went from 1.7s to 104ms by materializing it
in a CTE and joining against that instead. Any query that wants it per (account, date) has
this problem.

**A shared expense renders once per involved user.** Reading `expense_transactions_rendered`
without `user_id = ...` double-counts every transaction of an expense that touches two users'
accounts, since each user's rendering lists all of them. Filtering on `account_id` alone does
not save you — both copies name the same account.

**List queries take typed arguments, not filter documents.** The mongo version accepted a raw
filter document built in the browser, with the `userId` override merged in last. That is gone:
each list names its own sort enum and filter arguments, so a caller cannot reach a column the
resolver did not offer, and `user_id` is applied by the resolver alone.

**The login redirect keys on `extensions.code == "UNAUTHENTICATED"`**, which `require_user`
puts on its error and nothing else does. The guard in `+layout.svelte` redirects on that
alone, so that a backend that is down does not look like a session that ended. Both halves
ship together; a frontend newer than its backend simply never redirects.

**Password hashes are verified with the parameters from the stored PHC string**, not with
fixed ones. That is what keeps pre-2026 argonautica hashes (m=4096, t=32, p=num_cpus) working
alongside new argon2 ones. `verifies_legacy_parameters` in `lib.rs` guards this.

**An expense spans three tables.** `expenses`, `expense_transactions` and
`expense_categorizations`. `Expense::split()`/`join()` convert between the aggregate and the
rows; every write goes through `queries::insert_expense`/`replace_expense` inside a
`conn.transaction()`. Transaction order is stored as `idx` and has to stay dense, which is why
an update rewrites the children instead of diffing them.

**The changelog stores whole expenses as jsonb.** `expense_changes.old_state`/`new_state` are
snapshots on purpose: a later schema change must not retroactively alter what the history says
happened. `target_id` is deliberately not a foreign key — a change outlives its expense.

**TypeScript is pinned to 6 on purpose.** The 7.x Go port ships no CJS entry, so it crashes
`svelte-check` and `eslint`, and SvelteKit / svelte-check / typescript-eslint all cap their
peer range at `^6`. `npm outdated` will keep suggesting 7; don't take it until the toolchain
declares support.

**A rendered id is the upstream id.** A reader only ever sees their own rendering, so the view
returns the same value for both. What `graphql/rules.rs` still has to check is that the caller
can _see_ the expense a rule points at — a rule pointing at somebody else's expense would copy
money they cannot read.

**A modal inside a `<TabItem>` cannot close itself.** Writing to a `bind:open` prop from a
component rendered inside flowbite's tabs does not reach the parent, so the rules page closes
its modals from the parent's `onsubmit` callback instead. Modals on plain pages (categories,
accounts) close themselves and are fine as they are.

**Delivery dedup is amount + account + a 24h window, not statement text.** Banks reword the
text between provisional and final bookings. Widening the comparison would duplicate
expenses; narrowing it would drop real payments. There are tests for both directions.

**A shared expense's bank line is a fraction, not an amount.** In a split the booking is
`fraction = 1` of `reference_amount`, so `t.amount` is null on the very transaction that came
off a bank statement. Anything asking what a transaction is worth goes through
`calculated_amount` in SQL or `rules::resolved_amount` in Rust. Summing `amount` gives zero for
every shared expense, silently.

**A template must keep at least one transaction.** `expense_users` derives ownership from the
accounts an expense's transactions touch, so an expense with none belongs to nobody, renders in
no view and cannot be opened — which is why an automation template keeps a *blank booking* of
zero rather than dropping its payment line outright. The zero is what `impose` recognises;
position decides nothing.

**A rule may only reference an expense flagged `is_template`.** A past booking belongs in the
balances and totals. Flagging that booking as a template would silently remove real money from
both; use a separate template expense instead.

**`ExpenseFlagFilter` is an exclusion set, not a whitelist.** Each field is permission for that
flag to appear, so an expense lists only when *every* flag it carries is permitted. It can ask
for "templates and nothing else"; it cannot ask for "templates whatever else they are".
The rule picker uses the separate `onlyTemplates` argument for that reason.

**Live deliveries take their accrual period from the statement before the template.**
`accrual::statement_period` reads an explicit period from the bank's text. If none is present,
`accrual::Shape` uses the template's span and the phase of its blank booking to place the
delivery on its calendar grid. Calendar months must remain calendar months across short
months and daylight-saving changes.

**Opening 0,00 € balances are load-bearing: do not tidy them away.** An opening zero is
evidence that the account was empty at that instant, and it pins the left end of its history.
Remove it and the nearest anchor can become a later recorded balance, which back-integrates
every transaction in between and can show money before the account was opened. The empty
anchor `simulate_balances` offers at `-infinity` does not save you here: it is by construction
the furthest away, so a recorded balance always wins.

**The market value of a depot is an overlay, and the rule is that it never becomes a
balance.** `investments` reads renditier at request time and writes nothing: not to `balances`,
not to any other table, and not into the analysis payload. Keep it out of `analysis` — folding
it in there would put it inside the memoized keys in `cache.rs`, tie the frontend toggle to a
cache key, and make a renditier that is down or slow into a dashboard that is down or slow. Keep
it out of every *flow* as well (monthly totals, categories, the calendar, prepayments), toggle
or no toggle: the balances are deliberately conservative and booked about once a year, and
month-to-month spending is supposed to be about spending. On the frontend, the market figure is
offered on the start page and the Vermögen tab only, and there only when the Stichtag is today
— renditier prices what a depot is worth now, and laying that over a balance computed for a day
in 2019 answers nothing.

**The resolver must not hold a pool connection across the HTTP call**, for the reason above
about dataloaders, only more so: an outbound request is a far longer await than a loader batch.
`investments::list` reads the linked accounts and the token inside a block that ends before the
first request goes out; keep it that way.

**`GRAPHQL_API_URL` is vestigial.** It fed the server-side houdini client, which no longer
exists — the request url now comes from `houdini.config.js` and is relative. It is still set
in `docker-compose.yaml` and read by the frontend `dev` script; neither has any effect.

**`frontend/schema.graphql` is generated** by `cargo run -- schema > ../frontend/schema.graphql`
and is prettier-ignored. Don't hand-edit it. `houdini pull-schema` produces the same schema but
needs a running backend and therefore a database, which is a lot of setup for a file that
depends on neither; it also sorts arguments alphabetically, so switching between the two
reformats the whole file. Stay on `cargo run -- schema`.

**A job stuck in blocked with all of its dependencies green is a gitea problem, not a
workflow one.** `needs` is evaluated by a queue worker; when the `actions_ready_job` queue
stops draining, gitea keeps enqueuing "re-evaluate this run" and nothing acts on it, so every
job with a dependency waits until the `cancel_abandoned_jobs` cron cancels it. Nothing is
logged below Debug. Site Administration → Monitoring → Queues shows the item count; clearing
that queue is what fixed it in August 2026, after it had been broken since early 2025.

**The registry is `gitea.gerken.haus`, not `git.gerken.haus`.** The latter answers `/v2/`
with 403 and cost a red pipeline; every other app on that cluster pulls from the former.

**One CI run per branch.** `concurrency` with `cancel-in-progress` is set, so pushing again
cancels the run in flight, deploy included. A cancelled run is what a superseded commit looks
like here, not a failure to investigate.

**Component queries do not load themselves.** houdini 2 removed `@load`, the `_<Name>Variables`
convention and route-level `+page.gql` codegen. Every query needs an explicit
`$effect(() => void store.fetch())`, and for filtered tables the variables have to be passed
through the fetch call — that is what makes the effect re-run when a filter changes. Route
data is therefore client-side only; there is no SSR of query results.

## Conventions

- Indentation: rustfmt defaults in the backend, tabs in the frontend (`.prettierrc` sets
  `useTabs`). Just run the formatters.
- Comments explain _why_, not what changed. No changelog narration in source.
- **The UI is German; everything else is English.** Only what a reader sees on screen is
  German — labels, headings, messages, empty states. Identifiers, comments, commit messages,
  GraphQL field names, URL paths and query parameters are English: `?tab=activity`, not
  `?tab=aktivitaet`. A url is part of the interface a developer works with, not part of the
  copy. The exception is a value that _is_ the German text, such as a downloaded file's name.
- The frontend has a look, written down in [frontend/DESIGN.md](frontend/DESIGN.md): two
  typefaces and what each is for, what every colour means, how dates and money are written.
  Read it before adding a screen.
- Commit messages: imperative subject, body explaining the reasoning behind non-obvious
  choices. Don't restate the diff.
- Ask before switching branches or force-pushing.
- Dependency updates arrive as renovate PRs; digests and non-majors automerge, majors don't.

## Known gaps worth knowing before you plan work

- No component or end-to-end tests in the frontend; `npm test` is vitest over the pure
  modules under `src/lib` and nothing more. `tests/sql/` covers the views, the constraints and
  the balance simulation. DB-backed Rust tests cover scoped accessors and the real GraphQL
  authentication/key-lifecycle surface, not every domain resolver.
- No upload client feeds `deliveries.submit` from a real bank export; that is deliberately
  someone else's program, since it is per-bank and needs credentials.
- CORS is `AllowedOrigins::all()` with `allow_credentials(true)` in `graphql/mod.rs`, and
  `docker-compose.yaml` carries a committed `MONETA__API__SECRET_KEY`. Both are known and
  accepted for now — the deployment is private infrastructure.
- The value-date/accrual-period toggle is done and this is where it stops. Verlauf,
  the calendar and the dashboard donut share one `axisState`; the expense list picks one of the
  three dates for its column, sort and period filter. `simulate_balances`, `analysis_statement`
  and everything drawn from them — Kontostände, Vermögen, Kontoauszug — stay on the value date
  on purpose, because an accrued share is a cost a month carried rather than money in an
  account, and the *Fehler* column reconciles against recorded balances. Do not "finish the
  job" by putting a toggle on those. See [docs/analysis-dashboard.md](docs/analysis-dashboard.md).
