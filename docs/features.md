# What Moneta does

The user interface is German; everything a developer touches — identifiers, GraphQL fields,
URL paths, query parameters — is English. `?tab=activity`, not `?tab=aktivitaet`.

## The screens

| Route | Nav label | What it is |
| --- | --- | --- |
| `/` | — | the start page |
| `/analysis` | Auswertung | seven tabs of analysis, [documented separately](analysis-dashboard.md) |
| `/analysis/[year]/[month]` | — | one month in detail |
| `/expenses` | Ausgaben | the expense list |
| `/balances` | Kontostände | recorded balances |
| `/categories` | Kategorien | the category tree |
| `/accounts` | Konten | accounts, ownership and synchronization |
| `/rules` | Regeln | periodic and automation rules, in two tabs |
| `/deliveries` | Lieferungen | bank statement lines that have been handed in |
| `/login` | — | name and password |

`src/lib/navigation.ts` holds the navbar entries and is also where page titles come from, so
the two cannot drift apart.

## Start page

Opens on the number the whole application exists to produce: **net worth**, the change against
a month ago, and the curve that got there.

Below it, three things:

- **Was ansteht** — a short stack of cards, each a link to the thing it is talking about, and
  each hidden when there is nothing to say: enter a new expense, deliveries that no rule
  claimed, expenses flagged for review, walk through last month.
- **The running month** — a category donut with a toggle between spending and income, compared
  against the same point in the previous month. Carries the value-date/accrual toggle too, on
  desktop only.
- **The activity feed** — what changed since your last visit, falling back to the most recent
  changes when nothing is new (and saying so, since the two look identical otherwise).

"Since your last visit" comes from `users.visit`, a mutation the page opens with: it records
that you are here and returns when you were here before, and that second answer stops being
available the moment the first is written.

## Expenses

The list is the workhorse. Server-side **search, filtering and sorting** throughout — the
client names a column from a per-entity enum and a direction, and nothing on the wire is a
query language.

Filters: free-text search, accounts, categories, the five flags, and a period. The period
filter and the date column both follow whichever of the three dates is selected —
accrual, value or booking. Active filters show as removable chips.

Clicking a row opens the detail modal: the transactions with their per-account amounts, the
categorizations and their weights, receipts, positions, and the **changelog** with a rendered
diff of every change the expense has been through.

Creating and editing happen in a modal too. A transaction takes either an amount or a fraction
of the expense's reference amount, which is how a shared purchase is entered without doing the
division by hand.

## Balances

A balance is a reading somebody took, not a derived figure — see
[concepts.md](concepts.md#the-entities). This page is where they are entered and corrected.
The analysis reconciles its integrated totals against them, so a wrong one here shows up as a
*Fehler* on the statement and the monthly overview rather than being quietly absorbed.

## Accounts and categories

Accounts carry a kind, an **availability** (immediately … decades — how quickly the money can
be reached, which is the axis the wealth chart stacks on), a **risk**, an opening and closing
date, and an **ownership fraction**. Two accounts belonging to different users can be
**synchronized** so each user's books show the other's entry against their own account, with an
optional sign inversion; that is how a debt between two people is modelled. Sync is set up from
the CLI (`moneta account --sync`).

Categories are a tree, can carry a monthly limit, and an expense can be filed under several
with weights. Both accounts and categories carry a `usageCount` over the last year, which is
what the pickers sort on — the five accounts you actually use are five of dozens, and
alphabetical order buries them.

## Depots, and what they are worth today

A depot is an account like any other, and its balance is a reading somebody took on purpose —
which for a depot happens about once a year, at year end, with a conservative haircut on the
gains. That is deliberate: the monthly analysis is about spending habits, and a stock market
moving those figures around would ruin it as an answer to that question.

[stockdb](https://gitea.gerken.haus/thies/StockDB) knows what the same depots are worth at
today's prices. `accounts.stockdbAccountId` links the two, `users.stockdb_token` is the API
token that reads it (`moneta user --stockdb-token`), and `investments` — its own query, beside
`analysis` rather than inside it — is where the two numbers meet. It is an overlay and never a
write:

- nothing is stored, in `balances` or anywhere else
- it never enters `simulate_balances` or the analysis SQL, and it is not part of the memoized
  analysis payload, so a stockdb that is down costs one null rather than the dashboard
- no *flow* is offered it — monthly totals, categories, the calendar, prepayments — not even
  behind a toggle. Only the start page and the Vermögen tab, which are a stock of wealth right
  now, and on the Vermögen tab only for today's Stichtag: laying this afternoon's price over a
  balance computed for a day in 2019 answers nothing

The start page keeps the booked total in large type and puts the market figure under it as a
smaller line, marked `LIVE`, with the date of the price and a way over to StockDB. No toggle:
it only ever added a line, and a control that hides a number is one whose state you have to
remember. The Vermögen tab does have one, because there the market value *replaces* the booked
figure in the table and in the total, which is a choice rather than a disclosure.

An expanded depot on the accounts page adds what went in, the IRR, a link over to StockDB, and
the gain nobody has committed yet — the one figure the start page deliberately leaves out,
since that is where it is a number to act on.

The one path from a market value into the books is a button there that opens the ordinary
balance dialog with the figure filled in. What gets saved is still typed by hand.

## Rules

Two kinds, both of which create expenses by copying a **template** expense. Logic in
`backend/src/rules.rs`; the page holds both in tabs.

**Periodic rules** fire on a schedule (daily, weekly, monthly, yearly). Occurrences are
anchored on the template's `accrualStart`, so a monthly rule keeps its day of month however
long it lay dormant. `titleReplacement` substitutes `%d`/`%m`/`%y`. The amount is the
template's own: nothing else supplies one, and a rent that goes up is written where the rest
of the expense is written.

Generation runs from `moneta periodic --run`, the `periodicRules.run` mutation, or the button
on the page. In production it is a scheduled job (a k8s CronJob); the compose stack runs the
same command from a `periodic` sidecar so both paths stay identical. Deliberately **not** an
in-process timer: that would fire once per backend replica and hide its failures in the server
log. It is idempotent — a rule's `lastCreation` only advances once its expenses are stored,
and it stops at the first occurrence a template cannot produce rather than skipping past it,
so repairing the template resumes where the run left off instead of losing that month.

**Automation rules** match a delivered bank statement line on any combination of statement
regex, date regex, exact amount and account. The highest `priority` match wins, and its
template is copied with the delivered data imposed on it. The rule editor highlights what a
regex captures against real statement text as you type.

The delivery decides which account the money moved on, so one `REWE` rule serves the card
payment and the cash purchase alike. What the template contributes is the rest: the title, the
categories, the counter-bookings, the shares — and the *date* of its own booking, which says
how far into the period it pays for a payment falls. That is what puts a Kita fee debited on
the 30th into the month it pays for rather than the next one, and it is the only thing about a
payment neither the rule nor the delivery knows.

**A rule may only point at an expense flagged `isTemplate`**, enforced when a rule is saved.
Anything else is a real booking: it is in the balances and in the Auswertung, and a rule
pointing at one turns the rule dialog into a way of rewriting money that has already been
spent. MoneyDB allowed it and most of its rules did it, so the import gives each rule a copy of
its own instead. A template counts in no balance, no total and no analysis, and the expense
list hides them unless the *Vorlage* filter is switched on.

## Deliveries

Bank statement lines are handed in through the `deliveries.submit` mutation. Each line comes
back as one of four statuses:

| | |
| --- | --- |
| `CREATED` | a rule matched and an expense was created |
| `DUPLICATE` | already delivered before, nothing done |
| `UNMATCHED` | no rule matched; the line is remembered but produced no expense |
| `FAILED` | a rule matched but could not be applied — **not** remembered, so fixing the rule and re-uploading picks it up |

**Dedup is amount + account + a 24-hour window, and deliberately not the statement text**:
banks reword it between a provisional and a final booking of the same payment. The cost is that
a genuine second identical payment on the same day reads as a duplicate, which is the safer
way round to be wrong.

Every line is remembered whether or not a rule matched, so re-uploading an overlapping
statement is cheap. The page lists what has arrived with a **"Nur ohne Regel"** filter, and
that filtered list is the queue of rules still to be written.

**There is no upload client.** Nothing in this repository feeds `deliveries.submit` from a real
bank export; that is deliberately someone else's program, since it is per-bank and needs
credentials. The API, the matching and the seen-delivery tracking are done.

## Analysis

Seven tabs at `/analysis`, and its own document: **[analysis-dashboard.md](analysis-dashboard.md)**.

## Not built

Receipts in the UI, push notifications, the upload client above, and budget tracking for
recurring expenses. `TODO.md` is the current list.
