CREATE TYPE account_availability AS ENUM ('immediately', 'days',  'weeks', 'months', 'years', 'decades');
CREATE TYPE account_risk AS ENUM ('none', 'slight', 'low', 'medium', 'high', 'huge');
CREATE TYPE account_kind AS ENUM ('cash', 'debit', 'credit', 'debt', 'stocks', 'virtual', 'other');

CREATE TABLE accounts (
  id SERIAL PRIMARY KEY,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  description TEXT NOT NULL,
  color TEXT,
  iban TEXT,
  kind account_kind NOT NULL,
  availability account_availability NOT NULL,
  risk account_risk NOT NULL,
  hidden BOOLEAN NOT NULL
);
