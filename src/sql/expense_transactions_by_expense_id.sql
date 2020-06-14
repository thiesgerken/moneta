-- this query does not check if the user has access to the given expense id!
-- (this cannot be seen on this transaction alone, as all of the (synced) accounts might belong to other users)
select expense_transactions.*, account_synchronizations.* from expense_transactions join accounts on expense_transactions.account_id = accounts.id left join account_synchronizations on accounts.user_id != $1 and (accounts.id = account_synchronizations.account1 or accounts.id = account_synchronizations.account2)
   where expense_transactions.expense_id = $2
   order by id
