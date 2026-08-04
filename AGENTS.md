# Working on this repo

Read `README.md` first for what the project is and how the two-layer data model works. This
file is the operational stuff: what to run, and the traps that have already been stepped in.

## Verify your changes

```sh
cd backend  && cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test
cd frontend && pnpm build && pnpm check && pnpm lint
```

CI runs exactly these and is strict — `clippy -D warnings` and `fmt --check` both fail the
build, including on test code.

**Run `pnpm build` before `pnpm check` or `pnpm lint`.** The build triggers houdini codegen;
without it `$houdini` types are stale or missing and you will chase type errors that aren't
real. If codegen gets into a bad state, `rm -rf .houdini .svelte-kit` and rebuild.

## Never touch `volumes/mongo` directly

That directory is real personal financial data. It is gitignored and there is no other copy
of most of it.

To test against realistic data, copy it first and point a container at the copy:

```sh
cp -R volumes/mongo /tmp/scratch-mongo
docker run -d --name scratch -p 127.0.0.1:27099:27017 -v /tmp/scratch-mongo:/data/db mongo:8
MONETA__DATABASE__URL=mongodb://localhost:27099 cargo run -- <subcommand>
```

Note that starting the backend creates indexes, which is a write. The database is at
featureCompatibilityVersion 8.0; a `mongo:7` binary will refuse to open it.

## Traps

**Money is integer cents.** `amount: i32` everywhere upstream. Only fraction-derived values
(`calculatedAmount`, `effectiveAmount`) are `f64`. Do not introduce floats into stored
amounts.

**Rendering must stay in sync.** Any mutation of an upstream document has to re-render the
affected users' documents, or reads silently go stale. After changing anything in
`rendering.rs`, rebuild everything with `moneta account --rerender x` and re-run the tests —
they mutation-test the ownership-fraction and sync-inversion paths.

**`graphql/util.rs::merge_filter` is an authorization boundary.** Callers send raw MongoDB
filter documents as strings; the `userId` override is applied last so they cannot widen their
own scope. If you refactor it, keep the tests that assert a caller-supplied `userId` loses.

**Password hashes are verified with the parameters from the stored PHC string**, not with
fixed ones. That is what keeps pre-2026 argonautica hashes (m=4096, t=32, p=num_cpus) working
alongside new argon2 ones. `verifies_legacy_parameters` in `lib.rs` guards this.

**bson 3 datetime serde lives in `queries.rs`.** bson 3 removed
`mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime` in favour of `serde_with`
adapters; rather than annotate every model, there are local
`chrono_datetime_as_bson_datetime` / `optional_chrono_datetime_as_bson_datetime` modules.
Keep using those in `#[serde(with = ...)]` — the on-disk representation depends on it.

**TypeScript is pinned to 6 on purpose.** The 7.x Go port ships no CJS entry, so it crashes
`svelte-check` and `eslint`, and SvelteKit / svelte-check / typescript-eslint all cap their
peer range at `^6`. `pnpm outdated` will keep suggesting 7; don't take it until the toolchain
declares support.

**`moneta import` reads the export on stdin and matches users by name**, so every user in the
export must already exist in moneta or it panics. `--clear` wipes everything *except* users.

**`GRAPHQL_API_URL` is vestigial.** It fed the server-side houdini client, which no longer
exists — the request url now comes from `houdini.config.js` and is relative. It is still set
in `docker-compose.yaml` and read by the frontend `dev` script; neither has any effect.

**`frontend/schema.graphql` is generated** by `pnpm exec houdini pull-schema` (needs the
backend running) and is prettier-ignored. Don't hand-edit it, and don't reformat it.

**Component queries do not load themselves.** houdini 2 removed `@load`, the `_<Name>Variables`
convention and route-level `+page.gql` codegen. Every query needs an explicit
`$effect(() => void store.fetch())`, and for filtered tables the variables have to be passed
through the fetch call — that is what makes the effect re-run when a filter changes. Route
data is therefore client-side only; there is no SSR of query results.

## Conventions

- Indentation: rustfmt defaults in the backend, tabs in the frontend (`.prettierrc` sets
  `useTabs`). Just run the formatters.
- Comments explain *why*, not what changed. No changelog narration in source.
- Commit messages: imperative subject, body explaining the reasoning behind non-obvious
  choices. Don't restate the diff.
- Ask before switching branches or force-pushing.
- Dependency updates arrive as renovate PRs; digests and non-majors automerge, majors don't.

## Known gaps worth knowing before you plan work

- No frontend tests, no integration tests against a real mongo.
- Automation rules are not imported from MoneyDB (`import.rs:29`, struct commented out), so a
  migration drops them today.
- CORS is `AllowedOrigins::all()` with `allow_credentials(true)` in `graphql/mod.rs`, and
  `docker-compose.yaml` carries a committed `MONETA__API__SECRET_KEY`. Both are known and
  accepted for now — the deployment is private infrastructure.
- The dashboard shows only the changelog; the analysis views from MoneyDB were never ported.
