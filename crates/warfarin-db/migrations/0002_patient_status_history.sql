CREATE TABLE IF NOT EXISTS wf_patient_status_history (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    hn              TEXT NOT NULL,
    status          TEXT NOT NULL,
    reason          TEXT,
    effective_date  TEXT NOT NULL,
    created_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_wf_patient_status_history_hn_effective_date
    ON wf_patient_status_history (hn, effective_date DESC, id DESC);
