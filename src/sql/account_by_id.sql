select * from accounts left join account_synchronizations on accounts.id = account_synchronizations.account1 or accounts.id = account_synchronizations.account2 where accounts.id = $1

