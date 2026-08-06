# TODO

## must have

- upload client that feeds `deliveries.submit` from a bank export; could just be a python script.
  the api, matching rules and seen-delivery tracking are done (src/rules.rs).
- something to track recurring expenses (to show if they are missing / have been deducted twice or something like that)
  could also be implemented using budgets, maybe that makes more sense! ... but then we need another thing to track these, more fine-grained than categories -> category budgets to remember how much one is allowed to spend, warn based on that
- Change stream query endpoint
  API is kind of hard — who can see what? correct thing to do would be to persist uids who can see the changelog entry on each entry, depending on users that have access to either the old or the new state. Easier: just query all expenses that were updated recently and then query their changelogs?
- analysis dashboard like the old one — see [docs/analysis-dashboard.md](docs/analysis-dashboard.md).
  Vermögen and Vorauszahlungen read real data; Monate, Kategorien, Kontoauszug and Kalender are
  still mock. `analysis.overviews` and `analysis.statement` are the two endpoints left.

## would be nice

- push notifications
- ausgabe in transaktion umwandeln (zB Retoure mit original zusammenfassen, dann automatisch buchungszeitraum anpassen?)
