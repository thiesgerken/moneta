-- fails while a user has no password, which is the point: there is no hash to make up for them
ALTER TABLE users ALTER COLUMN password_hash SET NOT NULL;
