# The analysis dashboard

`/analysis`, a port of MoneyDB's `analysis` section. Every tab reads real data. This document
covers what each shows, the queries behind them, and the decisions that shaped both.

## The tabs

MoneyDB had seven. Five survive the port, because four of them were different views of the same
numbers. A sixth, **Aktivität**, is not from MoneyDB at all — it is the expense changelog,
which used to be the whole dashboard and now sits here, last, while the start page carries the
part of the feed worth arriving at.

| Tab | MoneyDB origin | Shows |
| --- | --- | --- |
| **Vermögen** | `computed_balances` + `history` | net worth over time (total, per account, or per availability), availability totals, per-account computed balances |
| **Verlauf** | `monthly_overviews` + `categories` + `sunburst` | bars per month or per year — income against expenses, or one of the two split by category — a table per period with each month linking to its own page, and a treemap of the whole range |
| **Kontoauszug** | `statements` | one account's ledger over a range: opening balance, expenses and recorded balances interleaved, running total, CSV export |
| **Kalender** | `calendar` | month grid, expenses per day with a daily sum |
| **Vorauszahlungen** | `prepayments` | what is still unconsumed at a chosen date, plus the separate list of incomplete expenses |
| **Aktivität** | — | the expense changelog with its diffs |

Merged away:

- **history** into **Vermögen** — the line chart and the balance table are the same simulation,
  once sampled over a range and once at a single date. Splitting them meant picking a date
  twice.
- **sunburst** into **Verlauf** — it was already a component of a tab, not a tab itself.
- **categories** into **Verlauf** — its bars were the Monate bars stacked by category instead
  of by side: same query, same x axis, same drilldown. A toggle says which, and the two tabs
  stop keeping a range each that had to be set twice to ask one question.

## The date axis

**Verlauf**, the calendar and the start page's donut share one toggle
(`axisState` in `src/lib/analysis/state.svelte.ts`) between the **value date** — when the money
moved — and the **accrual period** — when the cost was incurred. The expense list picks one of
the three dates for its column, its sort and its period filter.

Switching the axis is not a different `GROUP BY`. An expense accruing over six months belongs
to six of them in proportion, so the movement has to be **spread** before it is filed: an equal
share of the cost per day of the period, the same linear model `analysis.prepayments` reports a
remainder from. `analysis_movements(user, from, to, accrued)` is where that happens, and it is
also where the range filter lives — on the accrual axis a period reaching past the range
contributes only the months inside it, so the filter cannot stay on the transaction.

Months are the grain on both axes. A range ending mid-month keeps that month whole on the
accrual side, where the value side would cut it off at the day.

**`simulate_balances` and `analysis_statement` deliberately do not move**, and neither does
anything per account — Kontostände, the per-account and per-availability curves, Kontoauszug.
An accrued share is a cost a month carried, not money sitting in an account, and the *Fehler*
column reconciles totals against balances somebody recorded: there is nothing to reconcile a
sixth of a payment against. Do not "finish the job" by putting a toggle on those.

The **Gesamt** curve is the exception, and only because summing every account leaves no
recorded reading to contradict. It is not a second simulation either: `accrual_adjustment`
returns what to add to the value-date total at each sample, and the toggle picks which of the
two the one series draws. See [Vermögen](#vermögen) below.

## Vermögen

`analysis.balances(date)` for the table at a single instant, `analysis.history(from, to,
stepDays)` for the curve.

```graphql
type History {
  dates: [DateTime!]!
  byAccount: [AccountSeries!]!       # account + amounts, one per date
  byAvailability: [AvailabilitySeries!]!
  accruedTotal: [Float!]!            # everything, on the accrual axis
}
```

`history` repeats the `balances` computation at each sampled date. **`stepDays` comes from the
chart's visible range** rather than being fixed: zoomed out it lands around 30, zoomed all the
way in on 1. That is why the endpoint takes a resolution instead of returning everything — the
frontend asks for the density it can draw rather than downloading years of daily points to
throw most away. (MoneyDB's `pointspermonth` said the same thing more awkwardly, since it could
not express "one point per day" without knowing the month length.)

`byAvailability` is **per bucket, not cumulative**. Stacking per-bucket series gives the
cumulative reading back as the height of the stack; stacking cumulative ones would count the
liquid accounts once per level above them. MoneyDB returned the cumulative form because it drew
overlapping lines rather than a stack.

The **Gesamt** row counts every account, whichever ones the table is listing: the *Aktive
Konten* / *Alle* filter says which rows are worth reading, not what you own, and a wealth that
changed when you ticked a box would be a figure nobody could quote. What is left out therefore
gets a row of its own directly under it, and only when it is not zero.

`to` is always the last sample even when the step does not divide the range, otherwise the line
stops short of the right edge. The point count is **capped at 400 server-side** —
`from`/`to`/`stepDays` are user input and a decade at `stepDays: 1` is ~3650 simulations.
Exceeding the cap is an error rather than a coarser answer, because a series quietly returned
at a resolution nobody asked for is indistinguishable from one that was.

### The accrual axis, over everything

`accruedTotal` is the same curve read on the accrual period instead of the value date, and it
exists only as a total — the toggle appears in the **Gesamt** split and nowhere else. What it
adds is one term per transaction:

```
amount * (consumed(t) - [date < t])
```

the difference between what the accrual axis says the transaction contributes at *t* and what
the value axis says. A year paid in January is bought and not yet used, so the unconsumed part
comes back as wealth and melts away over the year; a half-year billed in July was used before
the money left, so it is gone before any account notices. Outside every open period the two
terms agree, the correction is exactly zero, and the two curves are one line — which is what
`tests/sql/accrual_wealth.sql` pins down, along with the convention that both sides change at
the instant *after* the event and never on it. An expense consumed on the day it was paid
cancels itself, and most transactions are that; they are dropped before the arithmetic
rather than multiplied by every sample.

`consumed` is the continuous form from [concepts.md](concepts.md#the-three-dates) — the one
`prepayments` reports a remainder from — not the whole-day month weights of
`analysis_movements`. This is sampled at instants, not bucketed into months.

**Only for a query that selects it.** The resolver looks at the selection rather than taking
an argument, so the start page's sparkline — which asks `history` for `byAccount` alone and
could not draw the other curve anyway — pays nothing, while the chart's own query always
carries the field and the toggle stays a client-side flip with no round trip. The flag is part
of the cache key, or an entry filled in for the sparkline would answer the chart with an empty
series.

What it costs when it is asked for, on the real books at 122 samples: ~50 ms over a two-year
window and ~130 ms over the whole history, against a simulation of 300–400 ms for the same
samples. The wider the window, the more accrual periods the range filter admits, so this grows
with the range where the simulation grows with the sample count.

### Chart behaviour

The wealth chart targets ~120 points and picks its step from the visible x-range. Zooming
rewrites the range, which **re-runs the query at the new resolution** rather than magnifying
points already drawn. Points get markers so individual samples are visible, and the hovered one
enlarges.

The default window is the last two years. Apex's own reset control is off and a labelled button
takes its place: apex's bails out unless *that chart instance* was zoomed interactively, and
since a resolution change rebuilds the chart, the instance behind the button never has a zoom
recorded.

Faint dashed verticals mark month starts, falling back to year starts beyond ~18 months so they
stay subtle rather than becoming a second grid. Tooltips carry their own formatter — by default
apex reuses the axis one, which rounds to `40k €`.

## Verlauf

Reads `analysis.overviews(from, to, accrued, granularity)`.

```graphql
type Overviews {
  periods: [PeriodOverview!]!        # per-period totals, per account and per category
  byCategory: [CategoryBreakdown!]!  # whole range, with its expenses, for the treemap
}

enum Granularity { MONTH, YEAR }
```

`from` and `to` **snap outwards to whole periods**, so asking for the middle of one gives you
that whole period. The balances at every period boundary are one `simulate_balances` call over
an array of period starts — a year of months costs one simulation, not thirteen.

**The grain is one parameter, not a second endpoint.** Bucketing happens in exactly one place,
`analysis_movements`, where `p_grain` is the word `date_trunc` takes. On the accrual axis it is
also the step of the `generate_series` that walks a cost across the periods it covers *and* the
length of the window each period's share is measured against — leave one of the three at a
month and a cost crossing New Year is measured against January. `tests/sql/analysis_granularity.sql`
is where that is pinned down.

Everything downstream already worked on whatever periods it was handed: `period_starts` in
Rust builds the boundaries, and the balance simulation samples them without caring how far
apart they are.

**The frontend picks the grain from the range and lets the toggle override it.** Above three
years it draws years — 120 monthly bars are more than the axis can label, so asking for a long
window is itself the request for a coarser grain. The toggle overrides that until the range
changes length again. Every bar and every table row opens the period's own page, at either
grain — see below.

**Amounts are `calculatedAmount`, not `effectiveAmount`.** The totals have to reconcile with
the balances, and those are account balances rather than ownership-weighted shares of them.
Mixing the two would make the integration error meaningless. It also means a half-owned account
contributes its whole movement here, which is the convention **Vermögen** already uses.

Per-period category totals live on `PeriodOverview` and feed the stacked bars, so **the
category breakdown needs no endpoint of its own** — it is a second reading of this one, which
is why it is a toggle rather than a tab. Its treemap comes from `byCategory`, which is the
whole range at once rather than a sum of the periods: the biggest expenses of a decade are not
the biggest of each of its years.

The table is the same table under either breakdown. A balance and an integration error are
facts about a period; a category has neither, so there is nothing to split there.

An expense split across categories contributes its weighted share to each, and one with no
categorization at all comes back under a null category. Dropping those would make the parts add
up to less than the whole, which is worse than a slice labelled "Ohne Kategorie".

### Integrationsfehler

`PeriodOverview` reports both `endTotal` (what the recorded balances say at the end of the
month) and the total reached by integrating expenses forward from the month's start. The gap
between them is MoneyDB's *Integrationsfehler*, and it is kept.

A large gap does **not** mean the arithmetic drifted — it means an expense is missing. That is
the whole point of the view.

### Period pages

`/analysis/[year]/[[month]]` is the same `overviews` query over a one-period range, plus an
expense list for the biggest few. `PeriodOverview` carries `byAccount`, which is the only thing
the page needs beyond what the table shows, so there is no endpoint of its own.

**One route at both grains.** Without a month in the path it is the year: the summary cards,
the category picture, the per-account breakdown and the biggest expenses are answers about a
period and none of them care how long it is. What a year has and a month does not is the table
of its months, each row linking to its own page.

That table is a **second `overviews` call** at the month grain, aliased into the same document
— not the twelve rows folded into one. The roll-up nets a transfer inside the period it
happened in, so a transfer leaving in May and landing in June nets over the year and not over
either month: summing the rows would report a year that never happened. Which is also why the
table says out loud that its months need not add up to the year above them.

Each period gets a route rather than an expandable row: summary cards, the category treemap, a
per-account breakdown and the biggest expenses. MoneyDB nested a second DataTable inside an
expanded row, which is the part that does not survive a narrow screen.

## Kontoauszug

`analysis.statement(accountId, from, to)`.

```graphql
type StatementEntry {
  kind: StatementEntryKind!          # OPENING | EXPENSE | BALANCE
  date: DateTime!                    # transaction date for expenses
  title: String!
  store: String!
  expenseId: UUID                    # null on balance rows
  amount: Float                      # this account's share in cents; null on balance rows
  balance: Float!                    # running total, re-anchored on recorded balances
  drift: Float                       # recorded minus carried, when a balance disagrees
}
```

**This is not the expenses page with a filter.** It is a reconciliation view, and three things
distinguish it:

- It **interleaves three sources** into one chronological ledger: the simulated opening balance
  at the range start, every expense touching the account, and the balances actually *recorded*
  for it.
- **Recorded balances re-anchor the running total.** Where one disagrees with the total carried
  into it, the ledger jumps, and that jump is the signal that an expense is missing. It is
  surfaced explicitly as `drift` rather than being allowed to pass silently — the same idea as
  the Integrationsfehler in **Verlauf**.
- The amount shown is the **account's own share**, not the expense total. Moneta gets this for
  free where MoneyDB did not: rendering already flattens sharing into per-account transactions,
  so the figure is just the sum of `calculatedAmount` over transactions naming the account.
  MoneyDB had to walk a separate `sharing` array.

It belongs on the server rather than being assembled by the client: the running total and the
drift have to be computed in one pass over a correctly ordered sequence, and that ordering is
exactly the sort of rule that goes wrong quietly when two clients each reimplement it.

**A balance is stated before the expenses of its own instant**, matching `balances_rendered`,
whose `date` includes expenses with `date <` it. Each recorded balance opens an island; within
one, the balance is the anchor plus the movements since it. That is what makes a recorded
balance override the total carried into it rather than be absorbed by it.

**Legs of one expense on one account at one instant merge into a single movement** — splitting
a purchase in two is an accounting detail, not two bank movements. Legs on different dates stay
separate rows, which is how a bank shows an instalment.

Sorted ascending to accumulate, returned descending, because newest first is what you want to
read. CSV export is part of the tab — that is how a range gets handed to anything outside
moneta.

MoneyDB's statement had a booking date next to the value date that its model did not carry.
Moneta's `expense_transactions.booking_date` is real but is deliberately used for nothing here;
see [concepts.md](concepts.md#the-three-dates).

## Vorauszahlungen

`analysis.prepayments(at)`.

```graphql
type Prepayment {
  expense: Expense!
  consumedFraction: Float!
  remaining: Float!                  # cents still unconsumed at `at`, signed like the expense
}
```

Only expenses with a non-degenerate accrual period that has not run out at `at` are returned.
`remaining` follows `effectiveAmount`, so it is the reader's share and carries the expense's
sign — negative for money that went out.

Ordered by `abs(remaining)`, descending: most still tied up first, whichever way it points.
The table does not show `remaining` in a column of its own — the bar beside the amount draws
the same fact, and the sum of it is in the card above — so the order is the only place the
figure is visible.

It returns the whole rendered expense rather than a copy of a few of its fields, so the view
can link to it and read its transactions: `paidOn` is `min(transaction.date)` and needs no
column of its own.

The prepayment machinery needs no special accounts or compensation flags because the accrual
period does the work; [moneydb.md](moneydb.md) covers what it replaced, and the import derives
real periods for the historical data rather than leaving everything degenerate.

**`isIncomplete` expenses appear on the same page but in their own table.** They are open items
too, but the missing amount is unknown, so there is no remaining balance to compute — only a
flag and what has been booked so far. Do not merge the two tables: a *Rest* column would be a
lie for the incomplete ones. That list needs no endpoint either — `expenses.list` with every
flag off except `incomplete`, alongside `normal: false`, is exactly that set.

## Kalender

No endpoint of its own: `expenses.list` with `valuedAfter`/`valuedBefore`, which filter on
transaction dates. The pre-existing `accruedAfter`/`accruedBefore` filter the accrual period,
which for a prepayment spans a year while its payment lands on one day — picking the wrong pair
is how a calendar ends up showing a rent payment on every day of the month.

## Implementation notes

All of the resolvers live in `backend/src/graphql/analysis.rs`, with the arithmetic in SQL
functions the migrations define so `tests/sql` can reach it: `analysis_movements`,
`simulate_balances`, `accrual_adjustment`, `analysis_statement`, `analysis_period_totals`,
`analysis_category_totals` and `analysis_category_expenses`.

**Do the work in SQL, not by folding in Rust.** That advice was written for mongo, where the
only alternative was an aggregation pipeline per sample date; postgres does the whole thing
set-based and `simulate_balances` is the worked example. Putting it in a SQL function rather
than a Rust query string is what makes it testable, and that has already caught a missing
template filter here plus three ordering and weighting bugs in the fixtures that review had
passed over.

Three traps, all covered at more length in [backend.md](backend.md#migrations-and-sql):
`expense_users` cannot be narrowed by account or date, so reading
`expense_transactions_rendered` per (account, date) rebuilds the whole rendering every time (a
120-point chart took 1.7 s that way and 104 ms once materialized); a shared expense renders once
per involved user, so a missing `user_id` filter double-counts; and a rule template carries a
real account and a real date, so **every one of these functions has to filter `is_template`
out**.

The analysis queries are memoized in process for ten minutes (`graphql/cache.rs`) and every
mutation clears the cache — see [backend.md](backend.md#three-things-that-bite) before adding
one.

## Charts

`apexcharts`, wrapped by `frontend/src/lib/Chart.svelte`. flowbite-svelte 1.x dropped the
`Chart` component that 0.x had, so there is nothing to reuse from the UI kit.

The treemap replaces MoneyDB's sunburst. Same category → expense hierarchy, but the outer ring
of a sunburst degenerates into unlabelable slivers at the sizes this data produces, and
apexcharts has no sunburst type anyway.
