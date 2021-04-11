CREATE TABLE expense_items (
  id SERIAL PRIMARY KEY,
  expense_id INTEGER NOT NULL REFERENCES expenses(id) ON DELETE CASCADE,
  amount BIGINT,
  units INTEGER NOT NULL,
  title TEXT NOT NULL,
  comments TEXT NOT NULL
)
