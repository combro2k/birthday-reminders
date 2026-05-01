-- Fix SQLite FK drift from users_old after UUID normalization migration
-- In SQLite, renaming a referenced table rewrites child FK metadata.
-- If user_reminder_settings is not recreated, it can keep referencing users_old.

ALTER TABLE user_reminder_settings RENAME TO user_reminder_settings_old;

CREATE TABLE IF NOT EXISTS user_reminder_settings (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    days_before TEXT NOT NULL DEFAULT '7,3,1,0'
);

INSERT INTO user_reminder_settings
SELECT
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
  days_before
FROM user_reminder_settings_old;

DROP TABLE user_reminder_settings_old;
