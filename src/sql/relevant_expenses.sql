select * from expenses
   where id in (select * from expense_range($1, $2, $3))
   order by id
