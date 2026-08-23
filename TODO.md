# TODO

## must have

- upload client that feeds `deliveries.submit` from a bank export; could just be a python script.
  the api, matching rules and seen-delivery tracking are done (src/rules.rs).
- the same for periodic rules: `instantiate` still shifts the template's span onto the occurrence,
  the way `impose` used to. `accrual::Shape` is what an automation rule now uses instead and it
  applies unchanged — a periodic rule knows the date it is generating for, which is all `period`
  wants. Left out of that change to keep it to one mechanism at a time.
- something to track recurring expenses (to show if they are missing / have been deducted twice or something like that)
  could also be implemented using budgets, maybe that makes more sense! ... but then we need another thing to track these, more fine-grained than categories -> category budgets to remember how much one is allowed to spend, warn based on that
- Change stream query endpoint
  API is kind of hard — who can see what? correct thing to do would be to persist uids who can see the changelog entry on each entry, depending on users that have access to either the old or the new state. Easier: just query all expenses that were updated recently and then query their changelogs?

## would be nice

- the date axis is done, and this is the record of where it stops. Monate, Kategorien, the
  calendar and the dashboard donut share one toggle (`axisState`); the expense list picks one of
  the three dates for its column, sort and period filter. `simulate_balances`,
  `analysis_statement` and everything drawn from them *per account* -- Kontostände, Kontoauszug,
  the per-account and per-availability curves -- stay on the value date on purpose, because an
  accrued share is a cost a month carried rather than money in an account, and the *Fehler*
  column reconciles against recorded balances. Do not "finish the job" by putting a toggle on
  those. The Gesamt curve does carry one, as a correction added onto the value-date total
  (`accrual_adjustment`), because summing every account leaves nothing recorded to contradict.
  See [docs/analysis-dashboard.md](docs/analysis-dashboard.md).
- push notifications
- ausgabe in transaktion umwandeln (zB Retoure mit original zusammenfassen, dann automatisch
  buchungszeitraum anpassen?). Der Import macht das inzwischen für die moneydb-Altlasten
  (`Chain` in `src/import.rs`); was fehlt, ist die Bedienung dafür im Frontend.
