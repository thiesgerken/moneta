# TODO

- importer: heuristic for automatic conversion of prepayments to booking periods?
- importer: are position counts incremented properly? e.g Brot, Zucchini, Zucchini, ...
- from time to time: maintenance on the database; there could be orphaned groups?
- clarify that position titles may not include commas

- edit/create/delete expenses, balances, categories, accounts; remember to do validation checks + creation of new expense events
- automation using templates and expense deliveries
- analysis
- push notifications
- front page: stream of recent events
- on big screens: expandable rows for expense table?

- Python API would be nice (access all rendered expenses?)

## Performance

- expense rendering in web could be improved: run the queries are done in parallel
