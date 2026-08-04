# Analysis dashboard

Port of MoneyDB's `analysis` section. A mock UI lives at `/analysis` (four tabs, hardcoded
numbers from `frontend/src/lib/analysis_mock.ts`); this document records what it should show
and what the backend still owes it.

## Displays

MoneyDB had seven tabs. Four survive the port, because several of them were different views
of the same numbers.

| Tab             | MoneyDB origin                                    | Shows                                                                                                                   |
| --------------- | ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| **Vermögen**    | `computed_balances` + `history`                   | net worth over time (total, or stacked per account), availability totals, and a per-account table of computed balances    |
| **Monate**      | `monthly_overviews`                               | income/expenses bar chart, plus a table per month: wealth start/end, delta, expense count, integration error              |
| **Kategorien**  | `categories` + `sunburst`                         | stacked bars per category per month, and a treemap of the whole period; toggle between expenses and income                |
| **Kalender**    | `calendar`                                        | month grid, expenses per day with a daily sum                                                                             |

Merged away:

- **history** into **Vermögen** — the line chart and the balance table are the same simulation,
  once sampled over a range and once at a single date. Splitting them meant picking a date twice.
- **sunburst** into **Kategorien** — it was already a component of that tab, not a tab itself.

Dropped:

- **Kontoauszüge** — the expenses page already filters by account and date. It is missing only a
  running-balance column, which belongs there rather than in a second table.
- **Vorauszahlungen** — blocked on the data model, see below.

### Integration error

`Monate` keeps MoneyDB's *Integrationsfehler*: the gap between the month-end balance computed by
integrating expenses forward from the month's start, and the balance actually measured. A large
gap does not mean the arithmetic drifted — it means an expense is missing. That is the whole
point of the view, so it stays.

### Prepayments are blocked

MoneyDB found prepayments through an `AccountKind = Prepayment` and cancelled them out with a
`Compensated` expense flag. moneta has neither: `AccountKind` is
Cash/Debit/Credit/Debt/Stocks/RealEstate/Virtual/Other, and there is no per-expense flag for
"this settles an earlier prepayment". Reviving the view needs a modelling decision first —
either add the kind and flag back, or express prepayments as a synchronized account pair and
detect settlement from the balance returning to zero.

Note that the imported data already has an account named *Vorauszahlungen*, so the concept
survived the import as an ordinary account; only the automatic detection is gone.

## Backend

`analysis.balances(date)` already exists (`backend/src/graphql/analysis.rs`) and covers the
balance table in **Vermögen**. Two endpoints are missing.

### `analysis.history(from, to, pointsPerMonth)`

```graphql
type History {
  dates: [DateTime!]!
  byAccount: [AccountSeries!]!       # accountId + amounts, one per date
  byAvailability: [AvailabilitySeries!]!
}
```

Repeats the `analysis.balances` computation at each sampled date. MoneyDB sampled at month
starts with an optional subdivision (`pointspermonth`), which is worth keeping — a year of
daily points is 365 simulations for a chart that cannot resolve them.

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
