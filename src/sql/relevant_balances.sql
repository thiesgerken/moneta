select balances.*, account_synchronizations.* from balances join accounts on accounts.id = balances.account_id left join account_synchronizations on accounts.user_id != $1 and (accounts.id = account_synchronizations.account1 or accounts.id = account_synchronizations.account2)
   where accounts.user_id = $1 or exists (select 1 from accounts as a1 where a1.user_id = $1 and (a1.id = account1 or a1.id = account2))
   order by balances.date, balances.account_id
   limit $2
   offset $3
