-- Warfarin Care: Local authentication (users + audit log).
-- Argon2id PHC strings in `users.password_hash`. No plaintext passwords
-- are ever written. The audit log records event type, username, success
-- flag, and an optional free-form `details` field - never passwords.

CREATE TABLE IF NOT EXISTS users (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    username        TEXT    NOT NULL UNIQUE,
    password_hash   TEXT    NOT NULL,
    role            TEXT    NOT NULL DEFAULT 'User',
    active          INTEGER NOT NULL DEFAULT 1,
    failed_attempts INTEGER NOT NULL DEFAULT 0,
    locked_until    TEXT,
    created_at      TEXT    NOT NULL,
    updated_at      TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS auth_audit_log (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type   TEXT    NOT NULL,
    username     TEXT    NOT NULL,
    success      INTEGER NOT NULL,
    occurred_at  TEXT    NOT NULL,
    details      TEXT
);

CREATE INDEX IF NOT EXISTS idx_auth_audit_log_occurred_at
  ON auth_audit_log (occurred_at);

CREATE INDEX IF NOT EXISTS idx_auth_audit_log_username
  ON auth_audit_log (username);
