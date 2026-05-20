-- Supabase Schema for Warfarin Care Cloud Sync
-- Source of Truth: SQLite migrations (0001-0008)
-- Last Updated: 2026-05-09

-- =====================================================
-- wf_patients (matches 0001_initial.sql + 0008_cloud_sync.sql)
-- =====================================================
DROP TABLE IF EXISTS wf_patients;
CREATE TABLE wf_patients (
    sync_id         UUID PRIMARY KEY,
    machine_id      TEXT NOT NULL,
    hn              TEXT NOT NULL,
    enrolled_at     TEXT NOT NULL,          -- ISO8601 datetime
    enrolled_by     TEXT,
    status          TEXT NOT NULL DEFAULT 'active',
    indication      TEXT,
    target_inr_low  REAL NOT NULL DEFAULT 2.0,
    target_inr_high REAL NOT NULL DEFAULT 3.0,
    notes           TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    deleted_at      TEXT,
    UNIQUE(hn)                             -- UNIQUE on hn for conflict resolution
);

CREATE INDEX idx_wf_patients_updated_at ON wf_patients(updated_at DESC);
CREATE INDEX idx_wf_patients_hn ON wf_patients(hn);
CREATE INDEX idx_wf_patients_machine_id ON wf_patients(machine_id);

-- =====================================================
-- wf_visits (matches 0001_initial + 0003 + 0004 + 0007 + 0008)
-- =====================================================
DROP TABLE IF EXISTS wf_visits;
CREATE TABLE wf_visits (
    sync_id                UUID PRIMARY KEY,
    machine_id             TEXT NOT NULL,
    hn                     TEXT NOT NULL,
    visit_date             TEXT NOT NULL,
    inr_value              REAL,
    inr_source             TEXT,
    current_dose_mgday     REAL,
    dose_detail            TEXT,
    new_dose_mgday         REAL,
    new_dose_detail        TEXT,
    new_dose_description   TEXT,
    selected_dose_option   TEXT,
    dose_changed          INTEGER NOT NULL DEFAULT 0,
    next_appointment      TEXT,
    next_inr_due           TEXT,
    physician              TEXT,
    notes                  TEXT,
    side_effects          TEXT,
    adherence              TEXT,
    created_by             TEXT,
    reviewed_at            TEXT,
    reviewed_by            TEXT,
    created_at            TEXT NOT NULL,
    updated_at            TEXT NOT NULL,
    deleted_at            TEXT
);

CREATE INDEX idx_wf_visits_updated_at ON wf_visits(updated_at DESC);
CREATE INDEX idx_wf_visits_hn ON wf_visits(hn);
CREATE INDEX idx_wf_visits_sync_id ON wf_visits(sync_id);

-- =====================================================
-- wf_dose_history (matches 0001_initial + 0008)
-- =====================================================
DROP TABLE IF EXISTS wf_dose_history;
CREATE TABLE wf_dose_history (
    sync_id          UUID PRIMARY KEY,
    machine_id       TEXT NOT NULL,
    hn               TEXT NOT NULL,
    changed_at       TEXT NOT NULL,
    old_dose_mgday   REAL,
    new_dose_mgday   REAL,
    old_detail       TEXT,
    new_detail       TEXT,
    reason           TEXT,
    inr_at_change    REAL,
    changed_by       TEXT,
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL,
    deleted_at       TEXT
);

CREATE INDEX idx_wf_dose_history_updated_at ON wf_dose_history(updated_at DESC);
CREATE INDEX idx_wf_dose_history_hn ON wf_dose_history(hn);

-- =====================================================
-- wf_appointments (matches 0001_initial + 0005 + 0008)
-- =====================================================
DROP TABLE IF EXISTS wf_appointments;
CREATE TABLE wf_appointments (
    sync_id                UUID PRIMARY KEY,
    machine_id             TEXT NOT NULL,
    hn                     TEXT NOT NULL,
    appt_date              TEXT NOT NULL,
    appt_type              TEXT,
    status                 TEXT NOT NULL DEFAULT 'scheduled',
    notes                  TEXT,
    source_visit_id        INTEGER,
    generated_from_visit   INTEGER NOT NULL DEFAULT 0,
    created_at            TEXT NOT NULL,
    updated_at            TEXT NOT NULL,
    deleted_at            TEXT
);

CREATE INDEX idx_wf_appointments_updated_at ON wf_appointments(updated_at DESC);
CREATE INDEX idx_wf_appointments_hn ON wf_appointments(hn);
CREATE INDEX idx_wf_appointments_appt_date ON wf_appointments(appt_date);

-- =====================================================
-- wf_outcomes (matches 0001_initial + 0008)
-- =====================================================
DROP TABLE IF EXISTS wf_outcomes;
CREATE TABLE wf_outcomes (
    sync_id       UUID PRIMARY KEY,
    machine_id    TEXT NOT NULL,
    hn            TEXT NOT NULL,
    event_date    TEXT NOT NULL,
    event_type    TEXT NOT NULL,
    description   TEXT,
    inr_at_event  REAL,
    action_taken  TEXT,
    created_by    TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    deleted_at    TEXT
);

CREATE INDEX idx_wf_outcomes_updated_at ON wf_outcomes(updated_at DESC);
CREATE INDEX idx_wf_outcomes_hn ON wf_outcomes(hn);

-- =====================================================
-- wf_patient_status_history (matches 0002 + 0008)
-- =====================================================
DROP TABLE IF EXISTS wf_patient_status_history;
CREATE TABLE wf_patient_status_history (
    sync_id          UUID PRIMARY KEY,
    machine_id       TEXT NOT NULL,
    hn               TEXT NOT NULL,
    status           TEXT NOT NULL,
    reason           TEXT,
    effective_date   TEXT NOT NULL,
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL,
    deleted_at       TEXT
);

CREATE INDEX idx_wf_patient_status_history_updated_at ON wf_patient_status_history(updated_at DESC);
CREATE INDEX idx_wf_patient_status_history_hn_effective_date ON wf_patient_status_history(hn, effective_date DESC);

-- =====================================================
-- RLS Policies (allow anon key full access - internal hospital tool)
-- =====================================================
ALTER TABLE wf_patients                ENABLE ROW LEVEL SECURITY;
ALTER TABLE wf_visits                  ENABLE ROW LEVEL SECURITY;
ALTER TABLE wf_dose_history           ENABLE ROW LEVEL SECURITY;
ALTER TABLE wf_appointments            ENABLE ROW LEVEL SECURITY;
ALTER TABLE wf_outcomes               ENABLE ROW LEVEL SECURITY;
ALTER TABLE wf_patient_status_history ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "anon_all" ON wf_patients;
DROP POLICY IF EXISTS "anon_all" ON wf_visits;
DROP POLICY IF EXISTS "anon_all" ON wf_dose_history;
DROP POLICY IF EXISTS "anon_all" ON wf_appointments;
DROP POLICY IF EXISTS "anon_all" ON wf_outcomes;
DROP POLICY IF EXISTS "anon_all" ON wf_patient_status_history;

CREATE POLICY "anon_all" ON wf_patients                FOR ALL TO anon USING (true) WITH CHECK (true);
CREATE POLICY "anon_all" ON wf_visits                  FOR ALL TO anon USING (true) WITH CHECK (true);
CREATE POLICY "anon_all" ON wf_dose_history           FOR ALL TO anon USING (true) WITH CHECK (true);
CREATE POLICY "anon_all" ON wf_appointments            FOR ALL TO anon USING (true) WITH CHECK (true);
CREATE POLICY "anon_all" ON wf_outcomes               FOR ALL TO anon USING (true) WITH CHECK (true);
CREATE POLICY "anon_all" ON wf_patient_status_history FOR ALL TO anon USING (true) WITH CHECK (true);

-- =====================================================
-- Note: synced_at column is LOCAL-ONLY in SQLite and is NOT synced to Supabase
-- =====================================================