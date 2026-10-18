# The data model

Moneta records what a household spends. Several people share it, some accounts belong to two
of them, and a single purchase can move money on more than one account at once. Almost every
design decision in the backend follows from those three facts.

## The entities

| Entity | What it is |
| --- | --- |
| **user** | someone who logs in. Owns accounts; owns no expenses directly |
| **account** | somewhere money sits: a bank account, cash, a depot, a debt. Has exactly one owner, a kind, an availability, a risk and an ownership fraction |
| **expense** | one event that moved money. Carries a title, a store, flags, an accrual period, a list of transactions and a list of categorizations |
| **transaction** | one leg of an expense: an account, a date, an amount. A purchase has one, a transfer two, a refunded order two on different dates |
| **balance** | a reading somebody took: this account held this much at this instant. Not derived — it is evidence, and the analysis reconciles against it |
| **category** | what an expense was for. Hierarchical, and an expense can be filed under several with weights |

Plus the machinery around them: **periodic rules** and **automation rules** create expenses
from a template, **seen deliveries** remember bank statement lines that have been handed in,
and **expense changes** are the changelog.

An expense belongs to nobody in particular. Who sees it follows from the accounts its
transactions touch, and there is no `user_id` column on `expenses` to be filtered on.

## Two layers

This is the part to understand before anything else.

**Upstream tables** — `expenses`, `expense_transactions`, `expense_categorizations`,
`balances`, `accounts`, `categories`, `users`, `expense_changes`. These are the truth, and
they are what a write touches.

**Rendered views** — `expenses_rendered`, `expense_transactions_rendered`, `balances_rendered`
and `expense_users`, defined in `migrations/*_rendered_views`. One row **per user per upstream
entry**. Every read path — GraphQL queries, the analysis functions — hits these and never the
upstream tables.

They exist because the question the application is actually asked is never "what did this
expense cost" but "what did it cost *me*", and the answer differs between two readers looking
at the same row.

### What rendering resolves

Three things, each of which would otherwise have to be redone at every call site:

- **Ownership fraction.** An account held jointly carries `ownership_fraction < 1`, and only
  that share of its movement counts towards its owner's wealth.
- **Fractional transactions.** A transaction carries *either* an `amount` in cents *or* a
  `fraction`, never both (a CHECK enforces it). A fraction resolves against the expense's
  `reference_amount` — the way to say "we split this three ways" without doing the division by
  hand. **In a split, the booking itself is a fraction too**, of 1: the money is then written
  down once, and correcting it moves the booking and every share together. Left as a fixed
  figure beside a reference amount that equals it, the two drift the moment either is edited.
  So an expense with a `reference_amount` has no transaction carrying that same amount — read
  what a transaction is worth from `calculatedAmount`, or `rules::resolved_amount` in Rust,
  rather than from `amount`.
- **Account synchronization.** Two accounts belonging to different users can be linked, so
  one person's entry appears in the other's books against *their* account. With `invert`, the
  sign flips on the way across, which is how a debt between the two users is modelled: what
  one records as money lent, the other reads as money owed.

### Two amounts come out

Every rendered transaction carries both, and mixing them up is the classic mistake here:

- **`calculatedAmount`** — the movement on the account, fraction resolved. What a bank
  statement would show, and what has to reconcile with recorded balances.
- **`effectiveAmount`** — how much the reader's own wealth moved: `calculatedAmount` weighted
  by the ownership fraction, and **zero** for a transaction on somebody else's account that
  the reader does not mirror.

Balances, the wealth curve and the monthly totals use `calculatedAmount`, because they have to
add up against balances that were actually measured. "What did this cost me" uses
`effectiveAmount`. An expense's own `effectiveAmount` is the sum over its transactions.

### Why views and not stored rows

Rendering is a pure function of the expense and the accounts it touches. There is nothing to
keep in sync, no write path, no rerender step to lose rows halfway through — an earlier
version deleted and reinserted rendered rows with no transaction around it, and could.

A rendered row keeps the id of the row it came from (`upstream_id`) and reports it as its own
`id` as well, since a reader only ever sees their own rendering of an entry and never has two
to tell apart.

`tests/sql/rendered_views.sql` is where the three rules above are pinned down. It mutation-
tests them, and four of its cases passed against an earlier and wrong version of the views.

## The three dates

An expense that was paid in July, posted by the bank in August and covers the whole of the
following quarter has three different dates, and conflating any two of them is a bug that
shows up months later as a number nobody can explain.

- **`date`** on a transaction — when the money moved (*Wertstellung*). Everything analytical
  is filed under this by default: balances, the wealth curve, the calendar, the statement.
- **`bookingDate`** on a transaction — when the bank posted the line to its own ledger
  (*Buchungstag*). Null unless a bank delivered it, and deliberately used for nothing: two
  statements of the same payment can disagree about it. It sits on the transaction rather
  than the expense because a bank posts a *line* — one account, one amount, one date — and a
  transfer has two legs posted by two banks on two days.
- **`accrualStart` / `accrualEnd`** on an expense — the period the expense is *for*
  (*Leistungszeitraum*). Never null: equal endpoints mean an ordinary expense, and a span
  means the cost is spread. A CHECK rejects a backwards period and `Expense::validate` runs on
  every write path.

The accrual period is the axis that makes prepayments work without special accounts. An
expense whose money left on one day but whose value is consumed over a year is just one with
`accrualEnd > accrualStart`; consumption is linear across the span, so at date *t*

```
consumed  = clamp((t - accrualStart) / (accrualEnd - accrualStart), 0, 1)
remaining = amount * (1 - consumed)
```

and a zero-length period degenerates to an ordinary expense, fully consumed the moment it
starts.

Most analysis views carry a **toggle** between the two axes (`axisState` in
`frontend/src/lib/analysis/state.svelte.ts`). The value date and the accrual period give two
defensible answers to "what did I spend in March", so the reader picks. Balances stay on the
value date and have no toggle — an accrued share is a cost a month carried, not money that sat
in an account, and the statement's *Fehler* column has to reconcile against readings someone
actually took.

The wealth *total* is the one thing derived from balances that does carry the toggle, because
over all accounts at once there is no recorded reading left to contradict. What it answers is
a different question: not what the accounts held, but what you were worth counting what you
had already paid for and not yet used, and what you had already used and not yet paid for. It
is computed as a correction added onto the value-date curve rather than as a second simulation
— `accrual_adjustment` in the migrations — so `simulate_balances` itself still knows only one
date. Per account, and per availability, there is no toggle.

## Flags on an expense

Not derived, not modelled as state machines, just five booleans that mean five specific things:

- **`isTemplate`** — a stanza for a rule to copy, never counted in any analysis. Every rule
  must point to an expense carrying this flag, not to a past booking. A template carries a
  real account and a real date, so nothing but the flag says it never happened: every analysis
  function filters it out, and forgetting to is invisible until a balance is wrong by exactly
  one template.
- **`isIncomplete`** — more transactions are expected and their size is not known yet. Unlike
  a prepayment there is no remainder to compute, which is why the two are listed separately.
- **`isUnchecked`** — a human should look at it: missing information, a low-confidence match.
- **`isTaxRelevant`** — pull it out when preparing tax forms.
- **`isExtraordinary`** — real, but should not count towards ordinary monthly spending.

## Money

Integer cents everywhere upstream: `amount: i32`, `reference_amount: i32`, `balances.amount:
i32`. The only `double precision` columns are genuine fractions — ownership shares,
categorization weights, transaction fractions. Values that come out of a fraction are `f64`
(`calculatedAmount`, `effectiveAmount`). Do not introduce a float into a stored amount.

A **negative amount is a spend**: amounts add to account balances, so income is positive and
a purchase is negative.

## Ids and enumerations

Ids are `uuidv7()`, defaulted by the database, which is why **PostgreSQL 18 is a hard
requirement** — the function does not exist before it. In normal operation the application
generates them; the default exists so fixtures and hand-written inserts do not have to.

Enumerations (account kind, availability, risk) are stored as text with a CHECK constraint
rather than as postgres enum types. Adding a variant stays a one-line migration instead of an
`ALTER TYPE`, and nothing has to map a custom type to Rust.

## Auditing

`expense_changes` records every write as a pair of whole-expense snapshots in `jsonb`. They
are snapshots on purpose: a later schema change must not retroactively alter what the history
says happened. `target_id` is deliberately not a foreign key, because a change outlives the
expense it describes.

The diff a reader sees is computed from the two snapshots when the changelog is read, and the
values travel to the frontend as JSON rather than as text the server wrote — only the reader
can say whether `-17900` is an amount, a weight or an id.
