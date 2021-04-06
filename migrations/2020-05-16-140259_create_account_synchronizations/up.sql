CREATE TABLE account_synchronizations (
  account1 INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  account2 INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  user1 INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  user2 INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  invert BOOL NOT NULL,
  PRIMARY KEY (account1),
  UNIQUE (account2),
  CHECK (account1 < account2)
)
