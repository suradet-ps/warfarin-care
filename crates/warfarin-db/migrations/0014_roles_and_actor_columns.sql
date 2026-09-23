-- Warfarin Care: four-role model + local actor accountability.
--
-- Migration 0011 wrote the legacy `User` role; the domain model now uses
-- Admin / Pharmacist / Clinician / Viewer, so rewrite the legacy value.
--
-- The two new columns are local-only on purpose. Cloud sync selects explicit
-- column lists, so they stay in SQLite until a future cloud schema migration
-- adds them to Supabase; nothing about the sync payload changes today.
UPDATE users
   SET role = 'Clinician'
 WHERE role = 'User';

ALTER TABLE wf_appointments ADD COLUMN created_by TEXT;

ALTER TABLE wf_patient_status_history ADD COLUMN changed_by TEXT;
