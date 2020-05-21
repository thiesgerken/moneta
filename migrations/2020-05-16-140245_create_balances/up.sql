CREATE TABLE balances (
  account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  date TIMESTAMPTZ NOT NULL,
  amount BIGINT NOT NULL,
  comment TEXT NOT NULL,
  PRIMARY KEY (account_id, date)
)
