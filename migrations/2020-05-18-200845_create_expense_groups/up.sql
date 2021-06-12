CREATE TABLE expense_groups (
  id SERIAL PRIMARY KEY,
  title TEXT NOT NULL,
  expected_member_count INTEGER NOT NULL
)
