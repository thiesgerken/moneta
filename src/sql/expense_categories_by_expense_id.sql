select expense_categories.*, category_replacements.* from expense_categories join categories on categories.id = expense_categories.category_id left join category_replacements on category_replacements.original = categories.id and category_replacements.user_id = $1
  where expense_categories.expense_id = $2
