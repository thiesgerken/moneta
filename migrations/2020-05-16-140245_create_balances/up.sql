CREATE TABLE balances (
  id SERIAL PRIMARY KEY,
  account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  date TIMESTAMPTZ NOT NULL,
  amount BIGINT NOT NULL,
  comment TEXT NOT NULL,
  UNIQUE (account_id, date)
)
