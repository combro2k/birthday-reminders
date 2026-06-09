CREATE TABLE IF NOT EXISTS unsubscribe_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_type TEXT NOT NULL,
    token TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ,
    used_at TIMESTAMPTZ,
    UNIQUE(user_id, channel_type)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_unsubscribe_tokens_token ON unsubscribe_tokens(token);
