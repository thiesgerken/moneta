CREATE TABLE expenses (
  id SERIAL PRIMARY KEY,
  title TEXT NOT NULL,
  description TEXT NOT NULL,
  store TEXT NOT NULL,
  comments TEXT NOT NULL,
  booking_start TIMESTAMPTZ NOT NULL,
  booking_end TIMESTAMPTZ NOT NULL,
  is_deleted BOOL NOT NULL,
  is_template BOOL NOT NULL,
  is_preliminary BOOL NOT NULL,
  is_tax_relevant BOOL NOT NULL,
  is_unchecked BOOL NOT NULL
)
