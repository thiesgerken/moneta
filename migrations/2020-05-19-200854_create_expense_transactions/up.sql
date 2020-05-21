CREATE TABLE expense_transactions (
  id SERIAL PRIMARY KEY,
  expense_id INTEGER NOT NULL REFERENCES expenses(id) ON DELETE CASCADE,
  account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  date TIMESTAMPTZ NOT NULL,
  amount BIGINT,
  percentage DOUBLE PRECISION,
  comment TEXT NOT NULL,
  statement TEXT NOT NULL,
  CHECK (num_nonnulls(amount, percentage) = 1)
)
