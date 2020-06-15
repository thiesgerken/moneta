select * from expense_receipts
    where expense_id in (select * from expense_range($1, $2, $3))
    order by expense_receipts.expense_id, expense_receipts.id
