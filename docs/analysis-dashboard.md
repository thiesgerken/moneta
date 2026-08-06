# Analysis dashboard

Port of MoneyDB's `analysis` section, at `/analysis`. All six tabs read real queries; the mock
is gone. This document records what each shows and the decisions behind it.

## Displays

MoneyDB had seven tabs. Six survive the port, because two of them were different views of
the same numbers.

| Tab                  | MoneyDB origin                  | Shows                                                                                                                 |
| -------------------- | ------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| **Vermögen**         | `computed_balances` + `history` | net worth over time (total, per account, or per availability), availability totals, per-account computed balances       |
| **Monate**           | `monthly_overviews`             | income/expenses bar chart and a table per month, each row linking to its own page                                       |
| **Kategorien**       | `categories` + `sunburst`       | stacked bars per category per month, and a treemap of the whole period; toggle between expenses and income              |
| **Kontoauszug**      | `statements`                    | one account's ledger over a range: opening balance, expenses and recorded balances interleaved, running total, CSV      |
| **Kalender**         | `calendar`                      | month grid, expenses per day with a daily sum                                                                           |
| **Vorauszahlungen**  | `prepayments`                   | what is still unconsumed at a chosen date, plus the separate list of incomplete expenses                                |

Merged away:

- **history** into **Vermögen** — the line chart and the balance table are the same simulation,
  once sampled over a range and once at a single date. Splitting them meant picking a date twice.
- **sunburst** into **Kategorien** — it was already a component of that tab, not a tab itself.

### Kontoauszug is not the expenses page with a filter

This was briefly written off as one, which was wrong. It is a reconciliation view, and three
things distinguish it:

- It **interleaves three sources** into one chronological ledger: the simulated opening balance
  at the range start, every expense touching the account, and the balances actually *recorded*
  for it.
- **Recorded balances re-anchor the running total.** Where a recorded balance disagrees with the
  total carried into it, the ledger jumps — and that jump is the signal that an expense is
  missing. The mock surfaces it explicitly as a badge rather than letting it pass silently,
  which is the same idea as the Integrationsfehler in **Monate**.
- The amount shown is the **account's own share**, not the expense total. moneta gets this for
  free where MoneyDB did not: rendering already flattens sharing into per-account transactions,
  so the relevant amount is just the sum of `calculatedAmount` over transactions naming the
  account. MoneyDB had to walk a separate `sharing` array.

CSV export is part of it — that is how a range gets handed to anything outside moneta.

### Month pages

Each month gets its own route (`/analysis/[year]/[month]`) rather than an expandable row:
summary cards, the category treemap, a per-account breakdown and the biggest expenses.
MoneyDB nested a second DataTable inside an expanded row, which is the part that does not
survive a narrow screen.

### Integration error

`Monate` keeps MoneyDB's *Integrationsfehler*: the gap between the month-end balance computed by
integrating expenses forward from the month's start, and the balance actually measured. A large
gap does not mean the arithmetic drifted — it means an expense is missing. That is the whole
point of the view, so it stays.

### Chart resolution

The wealth chart picks its step from the visible x-range, targeting ~120 points: zoomed out it
samples weekly or monthly, zoomed in it goes down to daily. Points get markers so individual
samples are visible, and the hovered one enlarges. Zooming rewrites the range, which re-runs
the query at the new resolution rather than magnifying the points already drawn.

The default window is the last two years. Apex's own reset control is off and a labelled button
takes its place: apex's bails out unless *that chart instance* was zoomed interactively, and
since a resolution change rebuilds the chart, the instance behind the button never has a zoom
recorded.

Faint dashed verticals mark month starts, falling back to year starts beyond ~18 months so
they stay subtle rather than becoming a second grid. Tooltips carry their own formatter — by
default apex reuses the axis one, which rounds to `40k €`.

This is why `history` takes a resolution rather than returning everything: the frontend asks
for the density it can draw, instead of downloading years of daily points to throw most away.

## Prepayments

Not blocked after all — the model already has the necessary axis, it is just unused.

`Expense` carries **`booking_start`/`booking_end`** (a period) while each `ExpenseTransaction`
carries its own **`date`** (when the cash moved). MoneyDB had only a single `valueDate`, which
is why it needed `AccountKind = Prepayment` plus a `Compensated` flag to net the two halves of
a prepayment against each other.

With the period, a prepayment needs no special machinery: it is an expense whose money left on
the transaction date and whose value is consumed across the booking period. **Consumption is
linear.** At date *t*:

```
consumed  = clamp((t - booking_start) / (booking_end - booking_start), 0, 1)
remaining = amount * (1 - consumed)
```

A zero-length period — which is what every imported expense currently has, since the importer
sets `booking_start` and `booking_end` to the same date — degenerates to an ordinary expense
that is fully consumed the moment it starts. So switching this on changes nothing until
somebody actually widens a booking period.

The same mechanism spreads an annual premium across twelve months instead of spiking one of
them, which is worth having in `Monate` independently of the prepayment view.

`is_incomplete` expenses appear on the same page but in their own table: they are open items
too, but the missing amount is unknown, so there is no remaining balance to compute — only a
flag and what has been booked so far. Do not merge the two tables; a `Rest` column would be
a lie for the incomplete ones.

`Expense::validate` already rejects a backwards period, and every write path runs it (the
GraphQL mutations, both rule kinds, the importer), so the accrual maths cannot be handed a
negative span. `backend/src/models/expense.rs` covers all three cases in tests.

## Transaction date vs booking period

Balances and most analysis follow **transaction dates** — that is when the money actually
moved, and it is what a bank statement agrees with. The booking period drives the prepayment
view and the accrual-based spread of costs across months.

Some views want both, so they should carry a toggle rather than a hardcoded choice. Which
ones is still open; the monthly overviews are the obvious first candidate, since "what did I
spend in March" has two defensible answers.

## Backend

All of it lives in `backend/src/graphql/analysis.rs`, with the arithmetic in SQL functions the
migrations define so that `tests/sql` can reach it: `simulate_balances`, `analysis_statement`,
`analysis_month_totals`, `analysis_category_totals` and `analysis_category_expenses`.

### `analysis.history(from, to, stepDays)` — done

```graphql
type History {
  dates: [DateTime!]!
  byAccount: [AccountSeries!]!       # account + amounts, one per date
  byAvailability: [AvailabilitySeries!]!
}
```

Repeats the `analysis.balances` computation at each sampled date. `stepDays` comes from the
chart's visible range rather than being fixed: zoomed out it lands on ~30, zoomed all the way
in on 1. MoneyDB's `pointspermonth` said the same thing more awkwardly, since it could not
express "one point per day" without knowing the month length.

`byAvailability` is **per bucket**, not cumulative. Stacking per-bucket series gives the
cumulative reading back as the height of the stack; stacking cumulative ones would count the
liquid accounts once per level above them. MoneyDB returned the cumulative form because it
drew overlapping lines rather than a stack.

`to` is always the last sample even when the step does not divide the range, otherwise the
line stops short of the right edge. The point count is capped at 400 server-side —
`from`/`to`/`stepDays` are user input and a decade at `stepDays: 1` is ~3650 simulations.
Exceeding it is an error rather than a coarser answer, because a series quietly returned at a
resolution nobody asked for is indistinguishable from one that was.

### `analysis.prepayments(at)` — done

```graphql
type Prepayment {
  expense: Expense!
  consumedFraction: Float!
  remaining: Float!                  # cents still unconsumed at `at`, signed like the expense
}
```

Returns the whole rendered expense rather than a copy of a few of its fields, so the view can
link to it and read its transactions — `paidOn` is `min(transaction.date)` and needs no
column of its own.

Only expenses with a non-degenerate booking period that has not run out at `at` are returned.
`remaining` follows `effectiveAmount`, so it is the reader's share and carries the expense's
sign (negative for money that went out).

The incomplete list needs no endpoint: `expenses.list` with every flag off except
`incomplete`, alongside `normal: false`, is exactly that set.

### `analysis.overviews(from, to)` — done

```graphql
type Overviews {
  months: [MonthOverview!]!          # per-month totals, per account and per category
  byCategory: [CategoryBreakdown!]!  # whole range, with its expenses, for the treemap
}
```

`from` and `to` snap outwards to whole months, so asking for the middle of one gives you that
whole month. The balances at every month boundary are one `simulate_balances` call over an
array of month starts — a year costs one simulation, not thirteen.

**Amounts are `calculatedAmount`, not `effectiveAmount`.** The totals have to reconcile with
the balances, and those are account balances rather than ownership-weighted shares of them.
Mixing the two would make the integration error meaningless. It also means a half-owned account
contributes its whole movement here, which is the same convention **Vermögen** already uses.

Per-month category totals live on `MonthOverview` and feed the stacked bars, so **Kategorien**
needs no endpoint of its own — it is a second reading of this one. Its treemap comes from
`byCategory`, which is the whole range at once rather than a sum of the months: the biggest
expenses of a year are not the biggest of each of its months.

An expense split across categories contributes its weighted share to each, and one with no
categorization at all comes back under a null category. Dropping those would make the parts add
up to less than the whole, which is worse than a slice labelled "Ohne Kategorie".

**Kalender** and the month pages' *Größte Ausgaben* need no endpoint either, but they do need
the right axis: `expenses.list` grew `valuedAfter`/`valuedBefore`, which filter on transaction
dates. The pre-existing `bookedAfter`/`bookedBefore` filter the booking period, which for a
prepayment spans a year while its payment lands on one day.

### Month pages are `overviews` over a single month

`MonthOverview` carries `byAccount` as well, which is the only thing the month page needs
beyond what the table shows. So `/analysis/[year]/[month]` is this same query with a one-month
range plus an expense list for the biggest few, rather than an endpoint of its own.

### `analysis.statement(accountId, from, to)` — done

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

It belongs on the server rather than being assembled by the client: the running total and the
drift have to be computed in one pass over a correctly ordered sequence, and that ordering is
exactly the sort of rule that goes wrong quietly when two clients each reimplement it.

**A balance is stated before the expenses of its own instant**, matching
`balances_rendered.date` including expenses with `date <` it. Each recorded balance opens an
island; within one, the balance is the anchor plus the movements since it. That is what makes a
recorded balance override the total carried into it rather than be absorbed by it, and `drift`
is the difference between the two.

**Legs of one expense on one account at one instant merge into a single movement** — splitting
a purchase in two is an accounting detail, not two bank movements. Legs on different dates stay
separate rows, which is how a bank shows an instalment.

Sort ascending to accumulate, return descending — newest first is what you want to read.

MoneyDB's statement had a booking date next to the value date. moneta does not model one:
`expense_transactions` carries only `date`. The column is gone rather than invented.

### Implementation note

Do the work in SQL, not by folding in Rust. That advice was written for mongo, where the only
alternative was an aggregation pipeline per sample date; postgres does the whole thing set-based
and `simulate_balances` in the migrations is the worked example.

Two traps are worth knowing before touching any of it.

Reading `expense_transactions_rendered` per (account, date) looks harmless and is not:
`expense_users` is a `UNION` over the whole transaction table and cannot be narrowed to one
account, so every lateral invocation rebuilds the rendering for all ~16k rows. A 120-point
chart took 1.7s that way and 104ms once the rendering was materialized once per call and
joined. The monthly totals sidestep it by being a single `GROUP BY` rather than a query per
month.

A **rule template** carries a real account and a real date — `instantiate` shifts its
transactions onto each occurrence — so nothing but `is_template` says it never happened. Every
one of these functions filters it out. Forgetting to is invisible until a balance is wrong by
exactly one template.

Put the arithmetic in a SQL function rather than a Rust string, so `tests/sql` can reach it.
That is the only place in the repo where a query can be tested against a database, and it is
worth the indirection: it caught the missing template filter in `analysis_statement` and, in
the fixtures, three ordering and weighting bugs that review had passed over.

## Charts

`apexcharts`, wrapped by `frontend/src/lib/Chart.svelte`. flowbite-svelte 1.x dropped the `Chart`
component that 0.x had, so there is nothing to reuse from the UI kit.

The treemap replaces MoneyDB's sunburst. Same category → expense hierarchy, but the outer ring of
a sunburst degenerates into unlabelable slivers at the sizes this data produces, and apexcharts
has no sunburst type anyway.
