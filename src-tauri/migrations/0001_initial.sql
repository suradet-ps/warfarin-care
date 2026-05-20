-- Warfarin Care: Initial Schema
CREATE TABLE IF NOT EXISTS wf_patients (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    hn                  TEXT NOT NULL UNIQUE,
    enrolled_at         TEXT NOT NULL,
    enrolled_by         TEXT,
    status              TEXT NOT NULL DEFAULT 'active',
    indication          TEXT,
    target_inr_low      REAL NOT NULL DEFAULT 2.0,
    target_inr_high     REAL NOT NULL DEFAULT 3.0,
    notes               TEXT,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS wf_visits (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    hn                  TEXT NOT NULL,
    visit_date          TEXT NOT NULL,
    inr_value           REAL,
    inr_source          TEXT,
    current_dose_mgday  REAL,
    dose_detail         TEXT,
    new_dose_mgday      REAL,
    new_dose_detail     TEXT,
    dose_changed        INTEGER NOT NULL DEFAULT 0,
    next_appointment    TEXT,
    next_inr_due        TEXT,
    physician           TEXT,
    notes               TEXT,
    side_effects        TEXT,
    adherence           TEXT,
    created_by          TEXT,
    created_at          TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS wf_dose_history (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    hn              TEXT NOT NULL,
    changed_at      TEXT NOT NULL,
    old_dose_mgday  REAL,
    new_dose_mgday  REAL,
    old_detail      TEXT,
    new_detail      TEXT,
    reason          TEXT,
    inr_at_change   REAL,
    changed_by      TEXT,
    created_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS wf_appointments (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    hn              TEXT NOT NULL,
    appt_date       TEXT NOT NULL,
    appt_type       TEXT,
    status          TEXT NOT NULL DEFAULT 'scheduled',
    notes           TEXT,
    created_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS wf_outcomes (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    hn              TEXT NOT NULL,
    event_date      TEXT NOT NULL,
    event_type      TEXT NOT NULL,
    description     TEXT,
    inr_at_event    REAL,
    action_taken    TEXT,
    created_by      TEXT,
    created_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS wf_settings (
    key     TEXT PRIMARY KEY,
    value   TEXT NOT NULL
);

INSERT OR IGNORE INTO wf_settings (key, value) VALUES
    ('hospital_name', 'Warfarin Care'),
    ('default_inr_af_low', '2.0'),
    ('default_inr_af_high', '3.0'),
    ('default_inr_dvt_low', '2.0'),
    ('default_inr_dvt_high', '3.0'),
    ('default_inr_pe_low', '2.0'),
    ('default_inr_pe_high', '3.0'),
    ('default_inr_mechanical_valve_low', '2.5'),
    ('default_inr_mechanical_valve_high', '3.5'),
    ('warfarin_icode_5mg', '1600014'),
    ('warfarin_icode_2mg', '1600013'),
    ('warfarin_icode_3mg', '1600024');
