# TODO

## must have

- upload client that feeds `deliveries.submit` from a bank export; could just be a python script.
  the api, matching rules and seen-delivery tracking are done (src/rules.rs).
- an automation rule should set the accrual period to the period the payment is *for*, not to a
  span of the same length starting at the delivery. `apply_automation_rule` shifts the template's
  span onto `delivery.date`, so the Grundsteuer demand that arrives on 15.08. accrues 15.08.–14.11.
  when it is the rates for July to September. A quarterly bill wants the quarter it names, and the
  bank usually prints it — `Rundfunk 07.2026 - 09.2026`, `GRUNDST 4VJ.24`, `ZEITRAUM 13.08.2018-
  13.08.2019`. The importer takes the same question from the other end, deriving the period from
  moneydb's monthly drawdowns (`Chain::accrual_period`); a rule has no drawdowns to read and needs
  either the statement or a period rule of its own. Until then a generated expense has a period of
  roughly the right length in roughly the wrong place, and the calendar and prepayment views show
  it there.
- something to track recurring expenses (to show if they are missing / have been deducted twice or something like that)
  could also be implemented using budgets, maybe that makes more sense! ... but then we need another thing to track these, more fine-grained than categories -> category budgets to remember how much one is allowed to spend, warn based on that
- Change stream query endpoint
  API is kind of hard — who can see what? correct thing to do would be to persist uids who can see the changelog entry on each entry, depending on users that have access to either the old or the new state. Easier: just query all expenses that were updated recently and then query their changelogs?

## would be nice

- decide which analysis views should offer a transaction-date / booking-period toggle. Today
  everything follows transaction dates; `expenses.list` has `valuedAfter`/`valuedBefore` for
  that axis and `accruedAfter`/`accruedBefore` for the other, so it is a UI question now.
  See [docs/analysis-dashboard.md](docs/analysis-dashboard.md).
- push notifications
- ausgabe in transaktion umwandeln (zB Retoure mit original zusammenfassen, dann automatisch
  buchungszeitraum anpassen?). Der Import macht das inzwischen für die moneydb-Altlasten
  (`Chain` in `src/import.rs`); was fehlt, ist die Bedienung dafür im Frontend.
