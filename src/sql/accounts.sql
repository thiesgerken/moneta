select * from accounts left join account_synchronizations on accounts.id = account_synchronizations.account1 or accounts.id = account_synchronizations.account2
   order by id
   limit $1
   offset $2
