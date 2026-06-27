ALTER TABLE wf_patients ADD COLUMN sync_id TEXT;
ALTER TABLE wf_patients ADD COLUMN machine_id TEXT;
ALTER TABLE wf_patients ADD COLUMN synced_at TEXT;
ALTER TABLE wf_patients ADD COLUMN deleted_at TEXT;

ALTER TABLE wf_visits ADD COLUMN sync_id TEXT;
ALTER TABLE wf_visits ADD COLUMN machine_id TEXT;
ALTER TABLE wf_visits ADD COLUMN synced_at TEXT;
ALTER TABLE wf_visits ADD COLUMN deleted_at TEXT;
ALTER TABLE wf_visits ADD COLUMN updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

ALTER TABLE wf_dose_history ADD COLUMN sync_id TEXT;
ALTER TABLE wf_dose_history ADD COLUMN machine_id TEXT;
ALTER TABLE wf_dose_history ADD COLUMN synced_at TEXT;
ALTER TABLE wf_dose_history ADD COLUMN deleted_at TEXT;
ALTER TABLE wf_dose_history ADD COLUMN updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

ALTER TABLE wf_appointments ADD COLUMN sync_id TEXT;
ALTER TABLE wf_appointments ADD COLUMN machine_id TEXT;
ALTER TABLE wf_appointments ADD COLUMN synced_at TEXT;
ALTER TABLE wf_appointments ADD COLUMN deleted_at TEXT;
ALTER TABLE wf_appointments ADD COLUMN updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

ALTER TABLE wf_outcomes ADD COLUMN sync_id TEXT;
ALTER TABLE wf_outcomes ADD COLUMN machine_id TEXT;
ALTER TABLE wf_outcomes ADD COLUMN synced_at TEXT;
ALTER TABLE wf_outcomes ADD COLUMN deleted_at TEXT;
ALTER TABLE wf_outcomes ADD COLUMN updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

ALTER TABLE wf_patient_status_history ADD COLUMN sync_id TEXT;
ALTER TABLE wf_patient_status_history ADD COLUMN machine_id TEXT;
ALTER TABLE wf_patient_status_history ADD COLUMN synced_at TEXT;
ALTER TABLE wf_patient_status_history ADD COLUMN deleted_at TEXT;
ALTER TABLE wf_patient_status_history ADD COLUMN updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

CREATE UNIQUE INDEX IF NOT EXISTS idx_wf_patients_sync_id
  ON wf_patients (sync_id)
  WHERE sync_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_wf_visits_sync_id
  ON wf_visits (sync_id)
  WHERE sync_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_wf_dose_history_sync_id
  ON wf_dose_history (sync_id)
  WHERE sync_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_wf_appointments_sync_id
  ON wf_appointments (sync_id)
  WHERE sync_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_wf_outcomes_sync_id
  ON wf_outcomes (sync_id)
  WHERE sync_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_wf_patient_status_history_sync_id
  ON wf_patient_status_history (sync_id)
  WHERE sync_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_wf_patients_sync_pending
  ON wf_patients (updated_at, synced_at, deleted_at);

CREATE INDEX IF NOT EXISTS idx_wf_visits_sync_pending
  ON wf_visits (updated_at, synced_at, deleted_at);

CREATE INDEX IF NOT EXISTS idx_wf_dose_history_sync_pending
  ON wf_dose_history (updated_at, synced_at, deleted_at);

CREATE INDEX IF NOT EXISTS idx_wf_appointments_sync_pending
  ON wf_appointments (updated_at, synced_at, deleted_at);

CREATE INDEX IF NOT EXISTS idx_wf_outcomes_sync_pending
  ON wf_outcomes (updated_at, synced_at, deleted_at);

CREATE INDEX IF NOT EXISTS idx_wf_patient_status_history_sync_pending
  ON wf_patient_status_history (updated_at, synced_at, deleted_at);