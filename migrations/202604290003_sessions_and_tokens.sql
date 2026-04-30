CREATE TABLE IF NOT EXISTS api_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_api_tokens_user_id ON api_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_api_tokens_token_hash ON api_tokens(token_hash);
