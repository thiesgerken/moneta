CREATE FUNCTION expense_range(p_user_id INTEGER, p_limit INTEGER, p_offset INTEGER)
 RETURNS TABLE (
		expense_id INTEGER
 ) AS $$
BEGIN

RETURN query select distinct expenses.id as expense_id from expenses inner join expense_transactions on expenses.id = expense_transactions.expense_id
              where expense_transactions.account_id in (select id from accounts where accounts.id in
                        (select account2 from account_synchronizations inner join accounts on accounts.id = account_synchronizations.account1 inner join users on accounts.user_id = users.id where users.id = p_user_id )
                    or accounts.id in
                        (select account1 from account_synchronizations inner join accounts on accounts.id = account_synchronizations.account2 inner join users on accounts.user_id = users.id where users.id = p_user_id )
                    or accounts.user_id = p_user_id )
              order by expenses.id
              limit p_limit
              offset p_offset;

END; $$
LANGUAGE PLPGSQL;

