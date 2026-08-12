# TODO

## must have

- upload client that feeds `deliveries.submit` from a bank export; could just be a python script.
  the api, matching rules and seen-delivery tracking are done (src/rules.rs).
- an automation rule should set the accrual period to the period the payment is *for*, not to a
  span of the same length starting at the delivery. `apply_automation_rule` shifts the template's
  span onto `delivery.date`, so the Grundsteuer demand that arrives on 15.08. accrues 15.08.–14.11.
  when it is the rates for July to September. A quarterly bill wants the quarter it names, and the
  bank usually prints it — `Rundfunk 07.2026 - 09.2026`, `GRUNDST 4VJ.24`, `ZEITRAUM 13.08.2018-
  13.08.2019`. Reading that off a statement is done: `MoneyDBExpense::statement_period` in
  `src/import.rs` parses all four spellings, and lifting it out of the import so
  `apply_automation_rule` can call it on `delivery.statement` is most of the work. What is left
  over — a monthly bill that names no period — is what the rule's own template has to say.
  Until then a generated expense has a period of roughly the right length in roughly the wrong
  place, and the calendar and prepayment views show it there.
- something to track recurring expenses (to show if they are missing / have been deducted twice or something like that)
  could also be implemented using budgets, maybe that makes more sense! ... but then we need another thing to track these, more fine-grained than categories -> category budgets to remember how much one is allowed to spend, warn based on that
- Change stream query endpoint
  API is kind of hard — who can see what? correct thing to do would be to persist uids who can see the changelog entry on each entry, depending on users that have access to either the old or the new state. Easier: just query all expenses that were updated recently and then query their changelogs?

## would be nice

- the date axis is done, and this is the record of where it stops. Monate, Kategorien, the
  calendar and the dashboard donut share one toggle (`axisState`); the expense list picks one of
  the three dates for its column, sort and period filter. `simulate_balances`,
  `analysis_statement` and everything drawn from them -- Kontostände, Vermögen, Kontoauszug --
  stay on the value date on purpose, because an accrued share is a cost a month carried rather
  than money in an account, and the *Fehler* column reconciles against recorded balances. Do not
  "finish the job" by putting a toggle on those.
  See [docs/analysis-dashboard.md](docs/analysis-dashboard.md).
- push notifications
- ausgabe in transaktion umwandeln (zB Retoure mit original zusammenfassen, dann automatisch
  buchungszeitraum anpassen?). Der Import macht das inzwischen für die moneydb-Altlasten
  (`Chain` in `src/import.rs`); was fehlt, ist die Bedienung dafür im Frontend.
