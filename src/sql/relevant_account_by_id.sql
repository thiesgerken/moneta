select * from accounts left join account_synchronizations on accounts.id = account_synchronizations.account1 or accounts.id = account_synchronizations.account2
   where (accounts.user_id = $1 or exists (select 1 from accounts as a1 where a1.user_id = $1 and (a1.id = account1 or a1.id = account2))) and accounts.id = $2

