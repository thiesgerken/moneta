# TODO

## must have

- expense delivery api, track seen expenses, upload client for that? could just be a python script though.
  matching rules exist now (src/rules.rs), they just need something to feed them.
- frontend for periodic and automation rules (backend + api are done)
- something to track recurring expenses (to show if they are missing / have been deducted twice or something like that)
  could also be implemented using budgets, maybe that makes more sense! ... but then we need another thing to track these, more fine-grained than categories -> category budgets to remember how much one is allowed to spend, warn based on that
- Change stream query endpoint
  API is kind of hard — who can see what? correct thing to do would be to persist uids who can see the changelog entry on each entry, depending on users that have access to either the old or the new state. Easier: just query all expenses that were updated recently and then query their changelogs?
- analysis dashboard like the old one

## would be nice

- push notifications
- ausgabe in transaktion umwandeln (zB Retoure mit original zusammenfassen, dann automatisch buchungszeitraum anpassen?)
