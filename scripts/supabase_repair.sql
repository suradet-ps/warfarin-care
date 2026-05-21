BEGIN;

-- Keep the newest patient row per HN before enforcing a global unique key.
WITH ranked_patients AS (
  SELECT
    sync_id,
    ROW_NUMBER() OVER (
      PARTITION BY hn
      ORDER BY updated_at DESC, created_at DESC, sync_id DESC
    ) AS row_num
  FROM wf_patients
)
DELETE FROM wf_patients
WHERE sync_id IN (
  SELECT sync_id
  FROM ranked_patients
  WHERE row_num > 1
);

DROP POLICY IF EXISTS anon_all_wf_patients ON wf_patients;
DROP POLICY IF EXISTS anon_all_wf_visits ON wf_visits;
DROP POLICY IF EXISTS anon_all_wf_dose_history ON wf_dose_history;
DROP POLICY IF EXISTS anon_all_wf_appointments ON wf_appointments;
DROP POLICY IF EXISTS anon_all_wf_outcomes ON wf_outcomes;
DROP POLICY IF EXISTS anon_all_wf_patient_status_history ON wf_patient_status_history;
DROP POLICY IF EXISTS anon_own_machine_wf_patients_select ON wf_patients;
DROP POLICY IF EXISTS anon_own_machine_wf_patients_insert ON wf_patients;
DROP POLICY IF EXISTS anon_own_machine_wf_patients_update ON wf_patients;
DROP POLICY IF EXISTS anon_own_machine_wf_visits_select ON wf_visits;
DROP POLICY IF EXISTS anon_own_machine_wf_visits_insert ON wf_visits;
DROP POLICY IF EXISTS anon_own_machine_wf_visits_update ON wf_visits;
DROP POLICY IF EXISTS anon_own_machine_wf_dose_history_select ON wf_dose_history;
DROP POLICY IF EXISTS anon_own_machine_wf_dose_history_insert ON wf_dose_history;
DROP POLICY IF EXISTS anon_own_machine_wf_dose_history_update ON wf_dose_history;
DROP POLICY IF EXISTS anon_own_machine_wf_appointments_select ON wf_appointments;
DROP POLICY IF EXISTS anon_own_machine_wf_appointments_insert ON wf_appointments;
DROP POLICY IF EXISTS anon_own_machine_wf_appointments_update ON wf_appointments;
DROP POLICY IF EXISTS anon_own_machine_wf_outcomes_select ON wf_outcomes;
DROP POLICY IF EXISTS anon_own_machine_wf_outcomes_insert ON wf_outcomes;
DROP POLICY IF EXISTS anon_own_machine_wf_outcomes_update ON wf_outcomes;
DROP POLICY IF EXISTS anon_own_machine_wf_patient_status_history_select ON wf_patient_status_history;
DROP POLICY IF EXISTS anon_own_machine_wf_patient_status_history_insert ON wf_patient_status_history;
DROP POLICY IF EXISTS anon_own_machine_wf_patient_status_history_update ON wf_patient_status_history;

CREATE POLICY anon_all_wf_patients ON wf_patients FOR ALL TO anon USING (true) WITH CHECK (true);
CREATE POLICY anon_all_wf_visits ON wf_visits FOR ALL TO anon USING (true) WITH CHECK (true);
CREATE POLICY anon_all_wf_dose_history ON wf_dose_history FOR ALL TO anon USING (true) WITH CHECK (true);
CREATE POLICY anon_all_wf_appointments ON wf_appointments FOR ALL TO anon USING (true) WITH CHECK (true);
CREATE POLICY anon_all_wf_outcomes ON wf_outcomes FOR ALL TO anon USING (true) WITH CHECK (true);
CREATE POLICY anon_all_wf_patient_status_history ON wf_patient_status_history FOR ALL TO anon USING (true) WITH CHECK (true);

CREATE UNIQUE INDEX IF NOT EXISTS idx_wf_patients_hn ON wf_patients(hn);

CREATE INDEX IF NOT EXISTS idx_wf_patients_updated_at ON wf_patients(updated_at);
CREATE INDEX IF NOT EXISTS idx_wf_visits_updated_at ON wf_visits(updated_at);
CREATE INDEX IF NOT EXISTS idx_wf_dose_history_updated_at ON wf_dose_history(updated_at);
CREATE INDEX IF NOT EXISTS idx_wf_appointments_updated_at ON wf_appointments(updated_at);
CREATE INDEX IF NOT EXISTS idx_wf_outcomes_updated_at ON wf_outcomes(updated_at);
CREATE INDEX IF NOT EXISTS idx_wf_patient_status_history_updated_at ON wf_patient_status_history(updated_at);

CREATE INDEX IF NOT EXISTS idx_wf_visits_hn ON wf_visits(hn);
CREATE INDEX IF NOT EXISTS idx_wf_dose_history_hn ON wf_dose_history(hn);
CREATE INDEX IF NOT EXISTS idx_wf_appointments_hn ON wf_appointments(hn);
CREATE INDEX IF NOT EXISTS idx_wf_outcomes_hn ON wf_outcomes(hn);
CREATE INDEX IF NOT EXISTS idx_wf_patient_status_history_hn ON wf_patient_status_history(hn);

-- Remote Supabase does not store the local SQLite visit `id`, so existing
-- `source_visit_id` values cannot be backfilled to `source_visit_sync_id`
-- safely on the server. This migration only prepares the schema for new syncs.
ALTER TABLE wf_appointments ADD COLUMN IF NOT EXISTS source_visit_sync_id UUID;
DROP INDEX IF EXISTS idx_wf_appointments_source_visit_id;
CREATE UNIQUE INDEX IF NOT EXISTS idx_wf_appointments_source_visit_sync_id
  ON wf_appointments(source_visit_sync_id)
  WHERE source_visit_sync_id IS NOT NULL;

COMMIT;