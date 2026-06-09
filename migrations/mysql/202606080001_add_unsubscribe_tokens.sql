CREATE TABLE IF NOT EXISTS unsubscribe_tokens (
    id CHAR(36) PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    channel_type VARCHAR(64) NOT NULL,
    token VARCHAR(255) NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    expires_at DATETIME(6) NULL,
    used_at DATETIME(6) NULL,
    UNIQUE KEY uq_unsubscribe_user_channel (user_id, channel_type),
    CONSTRAINT fk_unsubscribe_tokens_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

CREATE UNIQUE INDEX idx_unsubscribe_tokens_token ON unsubscribe_tokens(token);
