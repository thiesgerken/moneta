-- stockdb was renamed to renditier; the columns that link to it follow.
ALTER TABLE accounts RENAME COLUMN stockdb_account_id TO renditier_account_id;
ALTER TABLE users RENAME COLUMN stockdb_token TO renditier_token;
