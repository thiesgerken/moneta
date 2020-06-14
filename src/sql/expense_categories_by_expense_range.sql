select expense_categories.*, category_replacements.* from expense_categories join categories on categories.id = expense_categories.category_id left join category_replacements on category_replacements.original = categories.id and category_replacements.user_id = $1
  where expense_categories.expense_id in (select expenses.id from expenses inner join expense_transactions on expenses.id = expense_transactions.expense_id
              where expense_transactions.account_id in (select id from accounts where accounts.id in
                        (select account2 from account_synchronizations inner join accounts on accounts.id = account_synchronizations.account1 inner join users on accounts.user_id = users.id where users.id = $1 )
                    or accounts.id in
                        (select account1 from account_synchronizations inner join accounts on accounts.id = account_synchronizations.account2 inner join users on accounts.user_id = users.id where users.id = $1 )
                    or accounts.user_id = $1)
              order by expenses.id
              limit $2
              offset $3
            )
  order by expense_categories.expense_id
