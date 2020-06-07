CREATE TABLE category_replacements (
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  original INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
  replacement INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
  PRIMARY KEY (user_id, original)
)
