select * from expense_transactions
   where account_id in (select id from accounts where accounts.id in
            (select account2 from account_synchronizations inner join accounts on accounts.id = account_synchronizations.account1 inner join users on accounts.user_id = users.id where users.id = $1 )
		     or accounts.id in
            (select account1 from account_synchronizations inner join accounts on accounts.id = account_synchronizations.account2 inner join users on accounts.user_id = users.id where users.id = $1 )
		     or accounts.user_id = $1)
   order by expense_transactions.id
