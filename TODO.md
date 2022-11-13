# TODO

- analysis
- push notifications
- front page: stream of recent events
- on big screens: expandable rows for expense table?
- Python API would be nice (access all rendered expenses?)
- provide UpdateDTOs / CreateDTOs ?
- implement changelogs for balances
- implement changelogs for expenses
- would be awesome if effectiveAmount would be visible for other users as well in order to check correctness

## Performance

// TODO: Expense Delivery by API
// SeenExpense json sql=seen_expenses
// valueDate UTCTime
// bookingDate UTCTime Maybe
// amount Decimal
// accountId AccountId
// preliminary Bool
// transaction String
// deriving Show Generic
// |]

something to track recurring expenses (to show if they are missing / have been deducted twice or something like that)
...

category budgets to remember how much one is allowed to spend?
