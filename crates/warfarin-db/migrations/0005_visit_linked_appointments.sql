ALTER TABLE wf_appointments ADD COLUMN source_visit_id INTEGER;
ALTER TABLE wf_appointments ADD COLUMN generated_from_visit INTEGER NOT NULL DEFAULT 0;

CREATE UNIQUE INDEX IF NOT EXISTS idx_wf_appointments_source_visit_id
    ON wf_appointments (source_visit_id)
    WHERE source_visit_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_wf_appointments_hn_status_appt_date
    ON wf_appointments (hn, status, appt_date);
