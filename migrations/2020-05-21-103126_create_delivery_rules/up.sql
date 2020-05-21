CREATE TABLE delivery_rules (
  id SERIAL PRIMARY KEY,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  priority INTEGER NOT NULL,
  template_id INTEGER NOT NULL REFERENCES expenses(id) ON DELETE CASCADE,
  account_id INTEGER REFERENCES accounts(id) ON DELETE CASCADE,
  amount BIGINT,
  statement_regex TEXT NOT NULL,
  last_match TIMESTAMPTZ
)
