-- Warfarin Care: clinic identity for future multi-clinic deployments.
--
-- One generated UUID identifies this clinic. ensure_clinic_id backfills rows
-- created before the setting existed and stamps new enrollments and visits
-- from wf_settings. The column is local-only: cloud sync selects explicit
-- column lists, so Supabase payloads are unchanged.

ALTER TABLE wf_patients ADD COLUMN clinic_id TEXT;

ALTER TABLE wf_visits ADD COLUMN clinic_id TEXT;

CREATE INDEX IF NOT EXISTS idx_wf_patients_clinic_id ON wf_patients (clinic_id);

CREATE INDEX IF NOT EXISTS idx_wf_visits_clinic_id ON wf_visits (clinic_id);
