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

Filters: free-text search, accounts, categories, the five flags, and a period. The search
covers the text fields, the positions, the bank's statement text on each transaction, and an
amount in either sign. The period filter and the date column both follow whichever of the
three dates is selected — accrual, value or booking. Active filters show as removable chips.

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

The accrual period is measured, not shifted. A template spanning whole months spans whole
months wherever it lands, so a rule written as `01.01.–31.01.` covers February to the 28th
rather than to a fixed number of days into March; anything else keeps its span of days, which
is what a template booked across a handful of days wants. The booking itself does shift
rigidly, so a settlement booked on the 28th stays on the 28th.

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

An automation template holds a **blank booking**: one transaction of exactly zero, standing
where a payment goes. `impose` drops it and puts the delivered line in its place, so one `REWE`
rule serves the card payment and the cash purchase alike — the delivery decides the account and
the amount, and nothing on the template competes with it. The placeholder is recognised by its
zero rather than by position, so editing a template cannot quietly make a rule book a different
line. A template made by hand has no such line — you duplicate an expense and tick *Vorlage*,
and it keeps the booking it came from — so failing the placeholder the delivery replaces the
line on its own account, and failing that the first. It always replaces exactly one, or the
money would be booked twice: worst on a split, where a booking is written as `fraction = 1` and
a second one of those comes to 200 % of the bill without breaking any invariant.

It is kept rather than left out for two reasons. Its **date** is the phase — how far into the
period it pays for a payment falls, which is what puts a Kita fee debited on the 30th into the
month it pays for and is the one thing neither the rule nor the delivery knows. And an expense
with no transactions belongs to no user at all: `expense_users` derives ownership from the
accounts its transactions touch, so a template without one would render in no view and could
not be opened.

Everything else on the template is what the delivery gets wrapped in: the title, the categories,
the counter-bookings and the shares — **including `isUnchecked`**, the same way a periodic rule
inherits it. That flag is the rule author's answer to "is a delivery enough to finish this?".
For `Spotify` or the rent it is: the line resolves the whole expense and there is nothing to
confirm. For a supermarket it is not — the amount is right and the positions still want typing
in — so that template is ticked and every expense it produces arrives asking for attention.
Marking all of them unchecked would make the flag mean "arrived" rather than "needs you", and
then nobody reads it.

Three accounts are in play and they do different jobs:

| | |
| --- | --- |
| the **delivery's** account | decides where the generated expense books. Not a preference — the delivered line is added as it stands. |
| the **blank booking's** account | read by nothing on an automation template; it is there because a transaction has to name one and because that is what makes the template belong to somebody. On a *periodic* template the booking is not blank and its account is authoritative: there is no delivery, and nothing else names one. |
| the rule's **`filterAccountId`** | filters, and only filters: "only deliveries from this account". |

The accounts on counter-bookings and shares are real in both kinds: `Kreditkartenabrechnung`
genuinely books Commerzbank Girokonto against Commerzbank Kreditkarte. Only the blank booking is a placeholder.

So a template that serves deliveries on *any* account is written by setting its three dates and
leaving the rule's account filter empty. `accrualStart` gives the grid — its day of the month is
the anchor and its month puts a quarterly or half-yearly grid in phase — the span to
`accrualEnd` gives the length, and the blank booking's date minus `accrualStart` gives the
phase. Whichever account it names does not matter.

**A rule may only point at an expense flagged `isTemplate`**, enforced when a rule is saved.
Anything else is a real booking: it is in the balances and in the Auswertung, and a rule
pointing at one turns the rule dialog into a way of rewriting money that has already been
spent. MoneyDB allowed it and most of its rules did it, so the import gives each rule a copy of
its own instead. A template counts in no balance, no total and no analysis, and the expense
list hides them unless the *Vorlage* filter is switched on.

The rule editor's picker asks for templates through `onlyTemplates` rather than through the flag
filter. The flags are permissions — each one left off filters that flag *out* — so asking for
templates that way returns only those carrying no other flag, which on the imported books is 37
of 58 and leaves `REWE`, `KiTa-Gebühr` and every tax-relevant contract unpickable.

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

**A delivery is judged against `seen_deliveries` alone, never against the expenses that are
already there.** That is what makes the dedup cheap and predictable, and it means the table has
to be told about anything that arrived by another route — which is why the moneydb import seeds
it, see [moneydb.md](moneydb.md#the-import-says-what-the-bank-has-already-handed-over). Only the
last 400 days are ever compared.

**There is no upload client.** Nothing in this repository feeds `deliveries.submit` from a real
bank export; that is deliberately someone else's program, since it is per-bank and needs
credentials. The API, the matching and the seen-delivery tracking are done.

## Analysis

Seven tabs at `/analysis`, and its own document: **[analysis-dashboard.md](analysis-dashboard.md)**.

## Not built

Receipts in the UI, push notifications, the upload client above, and budget tracking for
recurring expenses. `TODO.md` is the current list.
