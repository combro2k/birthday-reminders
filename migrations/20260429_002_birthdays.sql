-- Birthdays table
CREATE TABLE IF NOT EXISTS birthdays (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    birth_date TEXT NOT NULL,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_birthdays_user_id ON birthdays(user_id);
CREATE INDEX IF NOT EXISTS idx_birthdays_birth_date ON birthdays(birth_date);

-- API tokens
CREATE TABLE IF NOT EXISTS api_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    last_used_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_api_tokens_user_id ON api_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_api_tokens_token_hash ON api_tokens(token_hash);

-- Notification channels
CREATE TABLE IF NOT EXISTS notification_channels (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_type TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    config TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(user_id, channel_type)
);

CREATE INDEX IF NOT EXISTS idx_notification_channels_user_id ON notification_channels(user_id);

-- User reminder settings
CREATE TABLE IF NOT EXISTS user_reminder_settings (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    days_before TEXT NOT NULL DEFAULT '7,3,1,0'
);

-- Reminder log
CREATE TABLE IF NOT EXISTS reminder_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    birthday_id TEXT NOT NULL REFERENCES birthdays(id) ON DELETE CASCADE,
    channel_type TEXT NOT NULL,
    reminded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    days_before INTEGER NOT NULL,
    year INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_reminder_log_birthday_id ON reminder_log(birthday_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_reminder_log_unique ON reminder_log(birthday_id, channel_type, days_before, year);
