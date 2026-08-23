# TODO

## must have

- periodic rules: `instantiate` still shifts the template's span onto the occurrence,
  the way `impose` used to. `accrual::Shape` is what an automation rule now uses instead and it
  applies unchanged — a periodic rule knows the date it is generating for, which is all `period`
  wants. Left out of that change to keep it to one mechanism at a time.
- something to track recurring expenses (to show if they are missing / have been deducted twice or something like that)
  could also be implemented using budgets, maybe that makes more sense! ... but then we need another thing to track these, more fine-grained than categories -> category budgets to remember how much one is allowed to spend, warn based on that
- Change stream query endpoint
  API is kind of hard — who can see what? correct thing to do would be to persist uids who can see the changelog entry on each entry, depending on users that have access to either the old or the new state. Easier: just query all expenses that were updated recently and then query their changelogs?

## would be nice

- push notifications
