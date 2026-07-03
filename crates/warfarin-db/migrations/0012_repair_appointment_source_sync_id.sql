-- Warfarin Care: repair legacy `wf_appointments.source_visit_sync_id`.
--
-- Background
-- ----------
-- Migration 0009 added `source_visit_sync_id` and backfilled it from
-- `wf_visits.sync_id` for every row that already had `source_visit_id` set.
-- For deployments that created visits **after** 0009 ran, the visit's
-- `sync_id` was still NULL at backfill time (sync_ids are minted lazily by
-- `ensure_sync_ids` before the first push). The backfill therefore left
-- `source_visit_sync_id = NULL` even though `source_visit_id` pointed at a
-- real visit.
--
-- That legacy state is now unsafe because the cloud-side push query
-- `COALESCE(a.source_visit_sync_id, v.sync_id)` falls back to the linked
-- visit's `sync_id` and can emit **two rows** that share the same
-- `source_visit_sync_id` (e.g. a properly-linked appointment plus a
-- stale same-`source_visit_id` row). Supabase then rejects the upsert
-- batch with HTTP 409 / Postgres `23505` on
-- `idx_wf_appointments_source_visit_sync_id`.
--
-- This migration repairs the data so every push-eligible row carries a
-- consistent `source_visit_sync_id` (or has its bogus link cleared).
--
-- Repair rules
-- ------------
-- 1. If `source_visit_id` is set and the linked visit exists AND the
--    appointment's `hn` matches the visit's `hn`, backfill
--    `source_visit_sync_id` from `wf_visits.sync_id`.
-- 2. If `source_visit_id` is set but the linked visit's `hn` does not
--    match (corrupt cross-patient link from an older code path), clear
--    both `source_visit_id` and `source_visit_sync_id`. The appointment
--    is left as a stand-alone record; a future visit-save flow can
--    re-link it.
-- 3. If `source_visit_id` is set but the linked visit no longer exists
--    (orphan FK), also clear the link.
--
-- A row that already has `source_visit_sync_id` set is left untouched.

-- 1. Healthy links: backfill the visit's sync_id.
UPDATE wf_appointments
   SET source_visit_sync_id = (
         SELECT v.sync_id
           FROM wf_visits v
          WHERE v.id = wf_appointments.source_visit_id
       )
 WHERE source_visit_id IS NOT NULL
   AND source_visit_sync_id IS NULL
   AND EXISTS (
         SELECT 1
           FROM wf_visits v
          WHERE v.id = wf_appointments.source_visit_id
            AND v.hn = wf_appointments.hn
       );

-- 2 & 3. Corrupt / orphan links: clear the link fields so the row no
-- longer participates in `source_visit_sync_id` uniqueness.
UPDATE wf_appointments
   SET source_visit_id = NULL,
       source_visit_sync_id = NULL
 WHERE source_visit_id IS NOT NULL
   AND source_visit_sync_id IS NULL;
