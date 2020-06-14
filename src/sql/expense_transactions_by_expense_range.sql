-- this query does not check if the user has access to the given expense id!
-- (this cannot be seen on this transaction alone, as all of the (synced) accounts might belong to other users)
select expense_transactions.*, account_synchronizations.* from expense_transactions join accounts on expense_transactions.account_id = accounts.id left join account_synchronizations on accounts.user_id != $1 and (accounts.id = account_synchronizations.account1 or accounts.id = account_synchronizations.account2)
   where expense_transactions.expense_id in (select expenses.id from expenses inner join expense_transactions on expenses.id = expense_transactions.expense_id
              where expense_transactions.account_id in (select id from accounts where accounts.id in
                        (select account2 from account_synchronizations inner join accounts on accounts.id = account_synchronizations.account1 inner join users on accounts.user_id = users.id where users.id = $1 )
                    or accounts.id in
                        (select account1 from account_synchronizations inner join accounts on accounts.id = account_synchronizations.account2 inner join users on accounts.user_id = users.id where users.id = $1 )
                    or accounts.user_id = $1)
              order by expenses.id
              limit $2
              offset $3
            )
   order by expense_transactions.expense_id, expense_transactions.id
