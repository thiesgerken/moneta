CREATE TABLE api_keys (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name text NOT NULL CHECK (name ~ '[^[:space:]]'),
    prefix text NOT NULL UNIQUE CHECK (prefix ~ '^[0-9a-f]{8}$'),
    secret_hash bytea NOT NULL CHECK (octet_length(secret_hash) = 32),
    access_level text NOT NULL CHECK (access_level IN ('read', 'write')),
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz CHECK (expires_at > created_at),
    last_used_at timestamptz
);
CREATE INDEX api_keys_user_id_idx ON api_keys(user_id);
