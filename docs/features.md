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

## Rules

Two kinds, both of which create expenses by copying a **template** expense. Logic in
`backend/src/rules.rs`; the page holds both in tabs.

**Periodic rules** fire on a schedule (daily, weekly, monthly, yearly). Occurrences are
anchored on the template's `accrualStart`, so a monthly rule keeps its day of month however
long it lay dormant. `titleReplacement` substitutes `%d`/`%m`/`%y`; `amountReplacement`
replaces the reference amount and, when the template has exactly one fixed transaction, that
transaction too — which is the shape almost every shared expense has.

Generation runs from `moneta periodic --run`, the `periodicRules.run` mutation, or the button
on the page. In production it is a scheduled job (a k8s CronJob); the compose stack runs the
same command from a `periodic` sidecar so both paths stay identical. Deliberately **not** an
in-process timer: that would fire once per backend replica and hide its failures in the server
log. It is idempotent — a rule's `lastCreation` only advances once its expenses are stored.

**Automation rules** match a delivered bank statement line on any combination of statement
regex, date regex, exact amount and account. The highest `priority` match wins, and its
template is copied with the delivered data imposed on it. The rule editor highlights what a
regex captures against real statement text as you type.

A template need not be flagged `isTemplate` — MoneyDB let ordinary past expenses double as
prototypes and the import preserved that.

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
