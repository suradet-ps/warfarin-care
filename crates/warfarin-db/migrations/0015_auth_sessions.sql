-- Warfarin Care: persistent sessions.
--
-- Only the SHA-256 hash of a session token is stored here; the raw token
-- lives in the OS keychain. A session stays valid while it is not revoked
-- and neither the idle timeout nor the absolute timeout has passed.

CREATE TABLE IF NOT EXISTS auth_sessions (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    token_hash    TEXT    NOT NULL UNIQUE,
    user_id       INTEGER NOT NULL,
    machine_id    TEXT,
    created_at    TEXT    NOT NULL,
    last_seen_at  TEXT    NOT NULL,
    expires_at    TEXT    NOT NULL,
    revoked_at    TEXT
);

CREATE INDEX IF NOT EXISTS idx_auth_sessions_user_id
  ON auth_sessions (user_id);

CREATE INDEX IF NOT EXISTS idx_auth_sessions_expires_at
  ON auth_sessions (expires_at);

ALTER TABLE users ADD COLUMN last_login_at TEXT;
