select * from expense_events
    where expense_id in (select * from expense_range($1, $2, $3))
    order by expense_events.expense_id, expense_events.id
