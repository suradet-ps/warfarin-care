-- Enrich wf_drug_interactions with severity, clinical_effect, management,
-- evidence_level columns for the Phase 1 interaction engine.

ALTER TABLE wf_drug_interactions ADD COLUMN severity TEXT NOT NULL DEFAULT 'moderate';
ALTER TABLE wf_drug_interactions ADD COLUMN clinical_effect TEXT;
ALTER TABLE wf_drug_interactions ADD COLUMN management TEXT;
ALTER TABLE wf_drug_interactions ADD COLUMN evidence_level TEXT;

-- Unified audit trail table. Every clinical action is logged here.
CREATE TABLE IF NOT EXISTS wf_audit_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    hn              TEXT,           -- NULL for system-level events (login, logout)
    action          TEXT NOT NULL,  -- visit_saved, dose_changed, status_changed, adverse_event, login, logout, etc.
    actor           TEXT NOT NULL,  -- username or 'system'
    timestamp       TEXT NOT NULL,  -- ISO 8601 (RFC 3339)
    old_value       TEXT,           -- JSON or text representation of previous state
    new_value       TEXT,           -- JSON or text representation of new state
    detail          TEXT,           -- JSON object with additional context
    created_at      TEXT NOT NULL
);

-- Index for efficient audit trail queries by patient and date range.
CREATE INDEX IF NOT EXISTS idx_audit_log_hn ON wf_audit_log(hn);
CREATE INDEX IF NOT EXISTS idx_audit_log_timestamp ON wf_audit_log(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_log_action ON wf_audit_log(action);
