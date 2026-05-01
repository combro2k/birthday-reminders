-- Normalize UUID columns from BLOB to TEXT format in SQLite
-- This handles existing data that may have been stored as BLOB when uuid::Uuid was bound directly
-- New inserts will use TEXT format via the updated Rust code

-- Helper: Safely convert UUIDs (handles both BLOB and TEXT already stored)
-- For BLOB (16 bytes), convert to standard UUID hex string with dashes
-- For TEXT, keep as-is

-- Table: users
ALTER TABLE users RENAME TO users_old;

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT,
    role TEXT NOT NULL DEFAULT 'user',
    auth_method TEXT NOT NULL DEFAULT 'local',
    oidc_subject TEXT UNIQUE,
    date_format VARCHAR(10) NOT NULL DEFAULT '%d-%m-%Y',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

INSERT INTO users 
SELECT 
  CASE 
    WHEN typeof(id) = 'blob' THEN PRINTF('%s-%s-%s-%s-%s',
      SUBSTR(HEX(id), 1, 8),
      SUBSTR(HEX(id), 9, 4),
      SUBSTR(HEX(id), 13, 4),
      SUBSTR(HEX(id), 17, 4),
      SUBSTR(HEX(id), 21, 12)
    )
    ELSE id
  END as id,
  username,
  email,
  password_hash,
  role,
  auth_method,
  oidc_subject,
  date_format,
  created_at,
  updated_at
FROM users_old;

DROP TABLE users_old;

CREATE INDEX IF NOT EXISTS idx_users_oidc_subject ON users(oidc_subject);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);

-- Table: birthdays
ALTER TABLE birthdays RENAME TO birthdays_old;

CREATE TABLE IF NOT EXISTS birthdays (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    birth_date DATE NOT NULL,
    phone_number TEXT,
    address TEXT,
    postal_code TEXT,
    city TEXT,
    country TEXT,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

INSERT INTO birthdays 
SELECT 
  CASE 
    WHEN typeof(id) = 'blob' THEN PRINTF('%s-%s-%s-%s-%s',
      SUBSTR(HEX(id), 1, 8),
      SUBSTR(HEX(id), 9, 4),
      SUBSTR(HEX(id), 13, 4),
      SUBSTR(HEX(id), 17, 4),
      SUBSTR(HEX(id), 21, 12)
    )
    ELSE id
  END as id,
  CASE 
    WHEN typeof(user_id) = 'blob' THEN PRINTF('%s-%s-%s-%s-%s',
      SUBSTR(HEX(user_id), 1, 8),
      SUBSTR(HEX(user_id), 9, 4),
      SUBSTR(HEX(user_id), 13, 4),
      SUBSTR(HEX(user_id), 17, 4),
      SUBSTR(HEX(user_id), 21, 12)
    )
    ELSE user_id
  END as user_id,
  name,
  birth_date,
  phone_number,
  address,
  postal_code,
  city,
  country,
  notes,
  created_at,
  updated_at
FROM birthdays_old;

DROP TABLE birthdays_old;

CREATE INDEX IF NOT EXISTS idx_birthdays_user_id ON birthdays(user_id);
CREATE INDEX IF NOT EXISTS idx_birthdays_birth_date ON birthdays(birth_date);

-- Table: api_tokens
ALTER TABLE api_tokens RENAME TO api_tokens_old;

CREATE TABLE IF NOT EXISTS api_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    last_used_at TEXT
);

INSERT INTO api_tokens 
SELECT 
  CASE 
    WHEN typeof(id) = 'blob' THEN PRINTF('%s-%s-%s-%s-%s',
      SUBSTR(HEX(id), 1, 8),
      SUBSTR(HEX(id), 9, 4),
      SUBSTR(HEX(id), 13, 4),
      SUBSTR(HEX(id), 17, 4),
      SUBSTR(HEX(id), 21, 12)
    )
    ELSE id
  END as id,
  CASE 
    WHEN typeof(user_id) = 'blob' THEN PRINTF('%s-%s-%s-%s-%s',
      SUBSTR(HEX(user_id), 1, 8),
      SUBSTR(HEX(user_id), 9, 4),
      SUBSTR(HEX(user_id), 13, 4),
      SUBSTR(HEX(user_id), 17, 4),
      SUBSTR(HEX(user_id), 21, 12)
    )
    ELSE user_id
  END as user_id,
  token_hash,
  name,
  created_at,
  last_used_at
FROM api_tokens_old;

DROP TABLE api_tokens_old;

CREATE INDEX IF NOT EXISTS idx_api_tokens_user_id ON api_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_api_tokens_token_hash ON api_tokens(token_hash);

-- Table: notification_channels
ALTER TABLE notification_channels RENAME TO notification_channels_old;

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

INSERT INTO notification_channels 
SELECT 
  CASE 
    WHEN typeof(id) = 'blob' THEN PRINTF('%s-%s-%s-%s-%s',
      SUBSTR(HEX(id), 1, 8),
      SUBSTR(HEX(id), 9, 4),
      SUBSTR(HEX(id), 13, 4),
      SUBSTR(HEX(id), 17, 4),
      SUBSTR(HEX(id), 21, 12)
    )
    ELSE id
  END as id,
  CASE 
    WHEN typeof(user_id) = 'blob' THEN PRINTF('%s-%s-%s-%s-%s',
      SUBSTR(HEX(user_id), 1, 8),
      SUBSTR(HEX(user_id), 9, 4),
      SUBSTR(HEX(user_id), 13, 4),
      SUBSTR(HEX(user_id), 17, 4),
      SUBSTR(HEX(user_id), 21, 12)
    )
    ELSE user_id
  END as user_id,
  channel_type,
  enabled,
  config,
  created_at,
  updated_at
FROM notification_channels_old;

DROP TABLE notification_channels_old;

CREATE INDEX IF NOT EXISTS idx_notification_channels_user_id ON notification_channels(user_id);

-- Table: reminder_log (no UUID in this table, but recreate for consistency)
ALTER TABLE reminder_log RENAME TO reminder_log_old;

CREATE TABLE IF NOT EXISTS reminder_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    birthday_id TEXT NOT NULL REFERENCES birthdays(id) ON DELETE CASCADE,
    channel_type TEXT NOT NULL,
    reminded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    days_before INTEGER NOT NULL,
    year INTEGER NOT NULL
);

INSERT INTO reminder_log 
SELECT 
  id,
  CASE 
    WHEN typeof(birthday_id) = 'blob' THEN PRINTF('%s-%s-%s-%s-%s',
      SUBSTR(HEX(birthday_id), 1, 8),
      SUBSTR(HEX(birthday_id), 9, 4),
      SUBSTR(HEX(birthday_id), 13, 4),
      SUBSTR(HEX(birthday_id), 17, 4),
      SUBSTR(HEX(birthday_id), 21, 12)
    )
    ELSE birthday_id
  END as birthday_id,
  channel_type,
  reminded_at,
  days_before,
  year
FROM reminder_log_old;

DROP TABLE reminder_log_old;

CREATE INDEX IF NOT EXISTS idx_reminder_log_birthday_id ON reminder_log(birthday_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_reminder_log_unique ON reminder_log(birthday_id, channel_type, days_before, year);
