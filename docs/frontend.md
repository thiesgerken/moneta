# Frontend

SvelteKit 2 with Svelte 5 in **runes mode** (forced), [houdini](https://houdinigraphql.com) 2 as
the GraphQL client, Tailwind 4 and flowbite-svelte for the UI, apexcharts for the charts.

What the UI is *supposed to look like* — the two typefaces, what each colour means, how dates
and money are written, and the handful of flowbite defaults that are overridden — is
**[../frontend/DESIGN.md](../frontend/DESIGN.md)**. Read it before adding a screen.

## Layout

The list pages are thin routes wrapping a table component, each paired with a modal for
create and edit:

```
src/routes/
  +page.svelte              the start page
  login/
  expenses/  balances/  categories/  accounts/  rules/  deliveries/
  analysis/                 the tabbed dashboard
  analysis/[year]/[month]/  one month
src/lib/
  *Table.svelte             one per list page
  *Modal.svelte             create/edit, one per entity
  ExpenseDetailModal.svelte + ExpenseChangelog.svelte + ChangeList.svelte
  Chart.svelte              the apexcharts wrapper
  navigation.ts             navbar entries and page titles
  money.ts  filters.ts  table_sort.svelte.ts  table_styles.ts
  analysis/                 the seven tabs and their shared state
```

`rules` holds both rule kinds in tabs. `analysis/` has one `*View.svelte` per tab plus
`state.svelte.ts`, which is where the shared range and date-axis selection live.

## Data fetching

**Component queries do not load themselves.** houdini 2 removed `@load`, the
`_<Name>Variables` convention and route-level `+page.gql` codegen. Every query is declared
inline and fetched explicitly:

```svelte
const store = graphql(`query Foo { ... }`);
$effect(() => void store.fetch());
```

For filtered tables the variables are computed from component state and passed through the
fetch call — that is what makes the effect re-run when a filter changes:

```svelte
$effect(() => void expenseStore.fetch({ variables: expenseVariables() }));
```

Route data is therefore **client-side only**; there is no SSR of query results, and that is a
deliberate non-goal rather than something waiting to be finished.

Filtering and sorting are **server-side** through typed arguments: each list query takes a
`search` string, whatever filters that entity has, and a `sortBy` enum plus `sortDir`.

The client url is relative (`/graphql`) because it ships in the browser bundle — the vite dev
server proxies it to `:8181`, traefik routes it in production. `MONETA__…`-style
`GRAPHQL_API_URL` is vestigial: it fed a server-side houdini client that no longer exists, and
neither the compose file nor the `dev` script that still set it have any effect.

`DateTime` and `UUID` are custom scalars, configured in `houdini.config.js`; `DateTime`
unmarshals to a real `Date`.

## Schema

`schema.graphql` is **generated and checked in**, prettier-ignored, and must not be
hand-edited:

```sh
cd backend && cargo run -- schema > ../frontend/schema.graphql
```

It is also the API reference — the backend's doc comments become descriptions in the SDL.

## Verifying

```sh
pnpm build && pnpm check && pnpm lint && pnpm test
```

**Run `pnpm build` before `check` or `lint`.** The build triggers houdini codegen; without it
the `$houdini` types are stale or missing and you will chase type errors that are not real. If
codegen gets into a bad state, `rm -rf .houdini .svelte-kit` and rebuild.

`pnpm test` is vitest over the plain modules under `src/lib` — the pure ones (`money.ts`,
`filters.ts`, `expense_utils.ts`, `table_sort`, `regex_highlight`), where the logic is worth
pinning down and no browser is needed. It has a `vitest.config.ts` of its own so it loads
neither houdini's plugin, whose watcher held every run open for ten seconds after the last
assertion, nor anything else the app needs only in order to build. **There are no component or
end-to-end tests.**

## Conventions worth knowing

- Indentation is **tabs** here (`.prettierrc` sets `useTabs`) and rustfmt defaults in the
  backend. Just run the formatters.
- **The UI is German; everything else is English** — labels, headings, messages and empty
  states are German, while identifiers, comments, GraphQL field names, URL paths and query
  parameters are not. A url is part of the interface a developer works with. The exception is a
  value that *is* the German text, such as a downloaded file's name.
- **TypeScript is pinned to 6 on purpose.** The 7.x Go port ships no CJS entry, so it crashes
  `svelte-check` and `eslint`, and SvelteKit, svelte-check and typescript-eslint all cap their
  peer range at `^6`. `pnpm outdated` will keep suggesting 7; don't take it.
- **A modal inside a `<TabItem>` cannot close itself.** Writing to a `bind:open` prop from a
  component rendered inside flowbite's tabs does not reach the parent, so the rules page closes
  its modals from the parent's `onsubmit` callback. Modals on plain pages close themselves and
  are fine as they are.
- The **login redirect** keys on `extensions.code == "UNAUTHENTICATED"` alone, so a backend that
  is down does not look like a session that ended.
