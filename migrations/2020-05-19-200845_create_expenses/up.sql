CREATE TABLE expenses (
  id SERIAL PRIMARY KEY,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  description TEXT NOT NULL,
  store TEXT NOT NULL,
  comments TEXT NOT NULL,
  booking_start TIMESTAMPTZ,
  booking_end TIMESTAMPTZ,
  booking_month_offset INTEGER,
  booking_month_count INTEGER,
  is_deleted BOOL NOT NULL,
  is_template BOOL NOT NULL,
  is_preliminary BOOL NOT NULL,
  is_tax_relevant BOOL NOT NULL,
  CHECK (num_nonnulls(booking_start, booking_month_count) = 1),
  CHECK (((booking_start IS NOT NULL) = (booking_end IS NOT NULL)) AND ((booking_month_count IS NOT NULL) = (booking_month_offset IS NOT NULL))),
  CHECK (booking_month_count IS NULL OR booking_month_count > 0)
)
