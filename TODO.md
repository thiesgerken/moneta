# TODO

- importer: heuristic for automatic conversion of prepayments to booking periods?
- from time to time: maintenance on the database; there could be orphaned groups?
- bench expense rendering with a db pool, so that the queries are done in parallel? If this reduces time, also do this in the production code

- edit/create/delete expenses, balances, categories, accounts; remember to do validation checks + creation of new expense events
- automation using templates and expense deliveries
- analysis
- push notifications
- front page: stream of recent events
- on big screens: expandable rows for expense table?
