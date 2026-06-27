ALTER TABLE wf_appointments ADD COLUMN source_visit_sync_id TEXT;

UPDATE wf_appointments
SET source_visit_sync_id = (
    SELECT wf_visits.sync_id
    FROM wf_visits
    WHERE wf_visits.id = wf_appointments.source_visit_id
)
WHERE source_visit_id IS NOT NULL
  AND source_visit_sync_id IS NULL;

DROP INDEX IF EXISTS idx_wf_appointments_source_visit_id;

CREATE UNIQUE INDEX IF NOT EXISTS idx_wf_appointments_source_visit_sync_id
    ON wf_appointments (source_visit_sync_id)
    WHERE source_visit_sync_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_wf_appointments_source_visit_id
    ON wf_appointments (source_visit_id)
    WHERE source_visit_id IS NOT NULL;