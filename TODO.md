# TODO

- expense delivery api, track seen expenses, client for that? could just be a python script though.
- something to track recurring expenses (to show if they are missing / have been deducted twice or something like that)
  could also be implemented using budgets, maybe that makes more sense! ... but then we need another thing to track these, more fine-grained than categories -> category budgets to remember how much one is allowed to spend, warn based on that
- better error handling
- analysis
- push notifications
- on big screens: expandable rows for expense table?
- Python API would be nice (access all rendered expenses?)
- provide UpdateDTOs / CreateDTOs ?
- Change stream query endpoint
  API is kind of hard — who can see what? correct thing to do would be to persist uids who can see the changelog entry on each entry, depending on users that have access to either the old or the new state. Easier: just query all expenses that were updated recently and then query their changelogs?
- would be awesome if effectiveAmount would be visible for other users as well in order to check correctness
