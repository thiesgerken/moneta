CREATE TYPE expense_event_type AS ENUM ('create', 'modify', 'delete');
CREATE TYPE expense_event_target AS ENUM ('expense', 'categories', 'receipts', 'transactions');

CREATE TABLE expense_events (
  id SERIAL PRIMARY KEY,
  expense_id INTEGER NOT NULL REFERENCES expenses(id) ON DELETE CASCADE,
  user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
  date TIMESTAMPTZ NOT NULL,
  tool TEXT NOT NULL,
  automatic BOOLEAN NOT NULL,
  event_type expense_event_type NOT NULL,
  event_target expense_event_target NOT NULL,
  payload TEXT
);
