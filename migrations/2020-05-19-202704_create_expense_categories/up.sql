CREATE TABLE expense_categories (
  expense_id INTEGER NOT NULL REFERENCES expenses(id) ON DELETE CASCADE,
  category_id INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
  weight DOUBLE PRECISION NOT NULL,
  PRIMARY KEY (expense_id, category_id),
  CHECK (weight > 0)
)
