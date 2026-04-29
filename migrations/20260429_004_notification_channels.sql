CREATE TABLE notification_channels (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_type TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    config JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, channel_type)
);

CREATE INDEX idx_notification_channels_user_id ON notification_channels(user_id);

CREATE TABLE user_reminder_settings (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    days_before INTEGER[] NOT NULL DEFAULT '{7, 3, 1, 0}'
);

CREATE TABLE reminder_log (
    id SERIAL PRIMARY KEY,
    birthday_id UUID NOT NULL REFERENCES birthdays(id) ON DELETE CASCADE,
    channel_type TEXT NOT NULL,
    reminded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    days_before INTEGER NOT NULL,
    year INTEGER NOT NULL
);

CREATE INDEX idx_reminder_log_birthday_id ON reminder_log(birthday_id);
CREATE UNIQUE INDEX idx_reminder_log_unique ON reminder_log(birthday_id, channel_type, days_before, year);
