-- Drug Interactions Management
CREATE TABLE IF NOT EXISTS wf_drug_interactions (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    icode               TEXT NOT NULL,
    drug_name           TEXT NOT NULL,
    strength            TEXT,
    interaction_type    TEXT NOT NULL,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL,
    UNIQUE(icode)
);

-- Separate settings into categories
-- Note: wf_settings already uses key-value, we just populate different keys
INSERT OR IGNORE INTO wf_settings (key, value) VALUES
    -- Hospital info (replacing single hospital_name key)
    ('hospital_name', 'Warfarin Care'),
    ('hospital_logo', ''),
    -- Default INR ranges by indication
    ('default_inr_af_low', '2.0'),
    ('default_inr_af_high', '3.0'),
    ('default_inr_dvt_low', '2.0'),
    ('default_inr_dvt_high', '3.0'),
    ('default_inr_pe_low', '2.0'),
    ('default_inr_pe_high', '3.0'),
    ('default_inr_mechanical_valve_low', '2.5'),
    ('default_inr_mechanical_valve_high', '3.5'),
    -- Warfarin drug codes
    ('warfarin_icode_5mg', '1600014'),
    ('warfarin_icode_2mg', '1600013'),
    ('warfarin_icode_3mg', '1600024');