-- A user can exist without a password: `moneta import` restores users without their
-- credentials, and they have none until `moneta user --update` gives them one. NULL says so,
-- and the login refuses it explicitly rather than relying on a placeholder failing to verify.
ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;
