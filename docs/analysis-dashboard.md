# Analysis dashboard

Port of MoneyDB's `analysis` section. A mock UI lives at `/analysis` (six tabs, hardcoded
numbers from `frontend/src/lib/analysis_mock.ts`); this document records what it should show
and what the backend still owes it.

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
samples monthly, zoomed in it goes down to daily. Points get markers so individual samples are
visible, and the hovered one enlarges. Zooming sets the range and the resolution follows; a
labelled reset button appears alongside apex's own toolbar once a zoom is active.

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

`analysis.balances(date)` already exists (`backend/src/graphql/analysis.rs`) and covers the
balance table in **Vermögen**. Three endpoints are missing.

### `analysis.history(from, to, stepDays)`

```graphql
type History {
  dates: [DateTime!]!
  byAccount: [AccountSeries!]!       # accountId + amounts, one per date
  byAvailability: [AvailabilitySeries!]!
}
```

Repeats the `analysis.balances` computation at each sampled date. `stepDays` comes from the
chart's visible range rather than being fixed: zoomed out it lands on ~30, zoomed all the way
in on 1. MoneyDB's `pointspermonth` said the same thing more awkwardly, since it could not
express "one point per day" without knowing the month length.

Cap the returned point count server-side too — `from`/`to`/`stepDays` are user input, and a
decade at `stepDays: 1` is ~3650 simulations.

### `analysis.prepayments(at)`

```graphql
type Prepayment {
  expenseId: ObjectIdWrapper!
  title: String!
  amount: Int!                       # cents, as booked
  paidOn: DateTime!                  # transaction date
  bookingStart: DateTime!
  bookingEnd: DateTime!
  consumedFraction: Float!
  remaining: Int!                    # cents still unconsumed at `at`
}
```

Returns only expenses with `remaining != 0`, so a zero-length booking period drops out on its
own. The incomplete list is just the expense query filtered on `isIncomplete`, so it needs no
endpoint.

### `analysis.overviews(from, to)`

```graphql
type Overviews {
  months: [MonthOverview!]!          # per-month balances, income, expenses, count, error
  byCategory: CategoryStats!         # whole-period tree, for the treemap
}
```

Per-month category stats live on `MonthOverview`, which is what feeds the stacked bars — so
**Kategorien** needs no endpoint of its own. **Kalender** needs none either; it is the existing
expense list filtered to a month.

### `analysis.statement(accountId, from, to)`

```graphql
type StatementEntry {
  kind: StatementEntryKind!          # OPENING | EXPENSE | BALANCE
  date: DateTime!                    # transaction date for expenses
  bookingDate: DateTime              # null while the bank has not booked it
  title: String!
  store: String!
  expenseId: ObjectIdWrapper         # null on balance rows
  amount: Int                        # this account's share in cents; null on balance rows
  balance: Int!                      # running total, re-anchored on recorded balances
  drift: Int                         # recorded minus carried, when a balance disagrees
}
```

Assemblable from what already exists — `analysis.balance(accountId, date)` for the opening row,
plus the rendered expense and balance collections filtered by account and range — so it could
start life in the frontend. It belongs on the server anyway: the running total and the drift
have to be computed in one pass over a correctly ordered sequence, and that ordering (value
date, balances after the expenses of the same instant) is exactly the sort of rule that goes
wrong quietly when two clients each reimplement it.

Sort ascending to accumulate, return descending — newest first is what you want to read.

### Implementation note

Load rendered balances and rendered expenses once and fold in Rust, the way MoneyDB's
`simulateCached` does — not one aggregation pipeline per sample date. The collections are small
(~16k rendered expenses, ~340 rendered balances), and every sampled date reuses the same working
set, so repeated round trips would dominate the cost.

## Charts

`apexcharts`, wrapped by `frontend/src/lib/Chart.svelte`. flowbite-svelte 1.x dropped the `Chart`
component that 0.x had, so there is nothing to reuse from the UI kit.

The treemap replaces MoneyDB's sunburst. Same category → expense hierarchy, but the outer ring of
a sunburst degenerates into unlabelable slivers at the sizes this data produces, and apexcharts
has no sunburst type anyway.
