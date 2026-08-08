# Working on this repo

Read `README.md` first for what the project is and how the two-layer data model works. This
file is the operational stuff: what to run, and the traps that have already been stepped in.
`docs/analysis-dashboard.md` covers the one part with enough decisions behind it to need its
own document, and `TODO.md` is what is deliberately not built yet.

## Verify your changes

```sh
cd backend  && cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test
cd backend  && PGURL=postgres://moneta:moneta@localhost:5433/moneta tests/run-sql-tests.sh
cd frontend && pnpm build && pnpm check && pnpm lint
```

The SQL tests need an empty postgres 18 (`docker compose up -d postgres` gives you one on
5433, but they apply the migrations themselves, so point them at a scratch database rather
than one holding real data).

CI runs exactly these and is strict — `clippy -D warnings` and `fmt --check` both fail the
build, including on test code.

A push to `main` also builds both images and restarts the deployments in the cluster, so a
merge is a deploy. `devel` builds images without deploying.

**Run `pnpm build` before `pnpm check` or `pnpm lint`.** The build triggers houdini codegen;
without it `$houdini` types are stale or missing and you will chase type errors that aren't
real. If codegen gets into a bad state, `rm -rf .houdini .svelte-kit` and rebuild.

## Never touch `volumes/postgres` directly

That directory is real personal financial data. It is gitignored and there is no other copy
of most of it.

To test against realistic data, use a throwaway container and re-import the moneydb export
into it:

```sh
docker run -d --name scratch -e POSTGRES_USER=moneta -e POSTGRES_PASSWORD=moneta \
    -e POSTGRES_DB=moneta -p 127.0.0.1:5499:5432 postgres:18-alpine
export MONETA__DATABASE__URL=postgres://moneta:moneta@localhost:5499/moneta
cargo run -- user --add thies    # every user in the export has to exist first
cargo run -- import --clear < moneydb-export.json
```

**Postgres 18 is a hard requirement**: the tables default their ids with `uuidv7()`, which
does not exist before 18.

## Traps

**Money is integer cents.** `amount: i32` everywhere upstream. Only fraction-derived values
(`calculatedAmount`, `effectiveAmount`) are `f64`. Do not introduce floats into stored
amounts.

**Rendering is a view, not stored state.** `expenses_rendered`, `expense_transactions_rendered`
and `balances_rendered` derive the per-user view from the upstream tables, so there is nothing
to keep in sync and no rerender step. If you change them, re-run `tests/sql/rendered_views.sql`:
it mutation-tests the ownership-fraction and sync-inversion paths, and four of its cases
passed against an earlier, wrong version of the views.

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
peer range at `^6`. `pnpm outdated` will keep suggesting 7; don't take it until the toolchain
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

**`moneta import` reads the export on stdin and matches users by name**, so every user in the
export must already exist in moneta or it panics. `--clear` wipes everything _except_ users.

**`GRAPHQL_API_URL` is vestigial.** It fed the server-side houdini client, which no longer
exists — the request url now comes from `houdini.config.js` and is relative. It is still set
in `docker-compose.yaml` and read by the frontend `dev` script; neither has any effect.

**`frontend/schema.graphql` is generated** by `cargo run -- schema > ../frontend/schema.graphql`
and is prettier-ignored. Don't hand-edit it. `houdini pull-schema` produces the same schema but
needs a running backend and therefore a database, which is a lot of setup for a file that
depends on neither; it also sorts arguments alphabetically, so switching between the two
reformats the whole file. Stay on `cargo run -- schema`.

**`needs:` does not work on this gitea, so the workflow has no job graph.** A job that
declares one stays blocked forever with every dependency green — measured with a probe job
that depended on a single succeeded job, against an otherwise identical job that ran.
Restarting gitea did not help. That is why the image build neither gates on the test jobs nor
lives in a job of its own: as a separate job it would have nothing holding it behind the
build and would deploy the previous run's image. Don't "fix" the missing `needs` without
checking whether the bug is gone.

**The registry is `gitea.gerken.haus`, not `git.gerken.haus`.** The latter answers `/v2/`
with 403 and cost a red pipeline; every other app on that cluster pulls from the former.

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

- No frontend tests. `tests/sql/` covers the views, the constraints and the balance
  simulation, but nothing exercises the GraphQL resolvers against a database.
- No upload client feeds `deliveries.submit` from a real bank export; that is deliberately
  someone else's program, since it is per-bank and needs credentials.
- CORS is `AllowedOrigins::all()` with `allow_credentials(true)` in `graphql/mod.rs`, and
  `docker-compose.yaml` carries a committed `MONETA__API__SECRET_KEY`. Both are known and
  accepted for now — the deployment is private infrastructure.
- `/analysis` reads real data throughout and `analysis_mock.ts` is gone. What is still open is
  which views should offer a value-date/accrual-period toggle — everything analytical
  follows transaction dates today, and `expenses.list` grew `valuedAfter`/`valuedBefore` for
  exactly that axis.
