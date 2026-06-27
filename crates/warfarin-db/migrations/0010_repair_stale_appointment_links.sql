UPDATE wf_appointments
SET source_visit_sync_id = (
    SELECT wf_visits.sync_id
    FROM wf_visits
    WHERE wf_visits.id = wf_appointments.source_visit_id
)
WHERE source_visit_id IS NOT NULL
  AND source_visit_sync_id IS NULL
  AND EXISTS (
    SELECT 1
    FROM wf_visits
    WHERE wf_visits.id = wf_appointments.source_visit_id
      AND wf_visits.hn = wf_appointments.hn
      AND wf_visits.next_appointment = wf_appointments.appt_date
  );

UPDATE wf_appointments
SET source_visit_id = NULL
WHERE source_visit_id IS NOT NULL
  AND source_visit_sync_id IS NULL
  AND EXISTS (
    SELECT 1
    FROM wf_visits
    WHERE wf_visits.id = wf_appointments.source_visit_id
      AND (
        wf_visits.hn != wf_appointments.hn
        OR COALESCE(wf_visits.next_appointment, '') != wf_appointments.appt_date
      )
  );