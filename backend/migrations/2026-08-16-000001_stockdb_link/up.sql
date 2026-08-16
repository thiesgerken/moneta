-- What ties a depot here to the same depot in stockdb, which knows what it is worth today.
--
-- The link is an explicit id rather than a match on `iban`: both sides have that column
-- nullable and a depot rarely carries one, so the obvious join would silently link nothing.
ALTER TABLE accounts
    ADD COLUMN stockdb_account_id integer;

-- stockdb issues one token per user and only stores its hash, so this is the token itself and
-- there is no way around that: it is presented on every call. It must never leave the backend
-- -- there is no GraphQL field for it, and `moneta user --stockdb-token` is how it gets set.
ALTER TABLE users
    ADD COLUMN stockdb_token text;
