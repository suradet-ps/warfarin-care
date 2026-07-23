-- Enrich wf_drug_interactions with severity, clinical_effect, management,
-- evidence_level columns for the Phase 1 interaction engine.

ALTER TABLE wf_drug_interactions ADD COLUMN severity TEXT NOT NULL DEFAULT 'moderate';
ALTER TABLE wf_drug_interactions ADD COLUMN clinical_effect TEXT;
ALTER TABLE wf_drug_interactions ADD COLUMN management TEXT;
ALTER TABLE wf_drug_interactions ADD COLUMN evidence_level TEXT;

-- Seed common warfarin drug interactions (clinic-wide default set).
-- The icode values below are placeholders — update them to match your
-- HosXP drugitems table. Clinicians can add/remove via Settings.
-- Severity: contraindicated / major / moderate / minor
-- Evidence: A = strong evidence, B = moderate, C = limited
INSERT OR IGNORE INTO wf_drug_interactions
  (icode, drug_name, strength, interaction_type, severity, clinical_effect, management, evidence_level, created_at, updated_at)
VALUES
  ('SEED_AMIODARONE', 'Amiodarone', NULL, 'increase', 'major',
   'Inhibits CYP2C9, increases warfarin effect significantly',
   'Reduce warfarin dose 30-50%, monitor INR weekly during initiation',
   'A', datetime('now'), datetime('now')),
  ('SEED_FLUCONAZOLE', 'Fluconazole', NULL, 'increase', 'major',
   'Potent CYP2C9 inhibitor, increases warfarin effect',
   'Reduce warfarin dose 25-50%, monitor INR closely',
   'A', datetime('now'), datetime('now')),
  ('SEED_METRONIDAZOLE', 'Metronidazole', NULL, 'increase', 'major',
   'Inhibits CYP2C9, increases warfarin effect',
   'Reduce warfarin dose 25-30%, monitor INR within 3-5 days',
   'A', datetime('now'), datetime('now')),
  ('SEED_RIFAMPICIN', 'Rifampicin', NULL, 'decrease', 'major',
   'Potent CYP inducer, decreases warfarin effect',
   'May need to double warfarin dose, monitor INR frequently',
   'A', datetime('now'), datetime('now')),
  ('SEED_PHENYTOIN', 'Phenytoin', NULL, 'increase', 'major',
   'Inhibits CYP2C9 at therapeutic levels, increases warfarin effect',
   'Monitor INR closely during initiation and discontinuation',
   'A', datetime('now'), datetime('now')),
  ('SEED_ASPIRIN', 'Aspirin', NULL, 'increase', 'major',
   'Increases bleeding risk via antiplatelet effect',
   'Avoid combination if possible; if necessary, use lowest dose with GI protection',
   'A', datetime('now'), datetime('now')),
  ('SEED_NSAID', 'NSAIDs', NULL, 'increase', 'moderate',
   'Antiplatelet effect and GI erosion increases bleeding risk',
   'Avoid if possible; use acetaminophen as alternative',
   'A', datetime('now'), datetime('now')),
  ('SEED_CARBAMAZEPINE', 'Carbamazepine', NULL, 'decrease', 'major',
   'Potent CYP inducer, decreases warfarin effect',
   'Monitor INR closely, warfarin dose may need significant increase',
   'A', datetime('now'), datetime('now')),
  ('SEED_OMEPRAZOLE', 'Omeprazole', NULL, 'increase', 'moderate',
   'Weak CYP2C9 inhibitor, may slightly increase warfarin effect',
   'Monitor INR when starting or stopping',
   'B', datetime('now'), datetime('now')),
  ('SEED_SIMVASTATIN', 'Simvastatin', NULL, 'increase', 'minor',
   'Minor interaction, may slightly increase INR',
   'Monitor INR periodically',
   'B', datetime('now'), datetime('now'));

-- Unified audit trail table. Every clinical action is logged here.
CREATE TABLE IF NOT EXISTS wf_audit_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    hn              TEXT,           -- NULL for system-level events (login, logout)
    action          TEXT NOT NULL,  -- visit_saved, dose_changed, status_changed, adverse_event, login, logout, etc.
    actor           TEXT NOT NULL,  -- username or 'system'
    timestamp       TEXT NOT NULL,  -- ISO 8601 (RFC 3339)
    old_value       TEXT,           -- JSON or text representation of previous state
    new_value       TEXT,           -- JSON or text representation of new state
    detail          TEXT,           -- JSON object with additional context
    created_at      TEXT NOT NULL
);

-- Index for efficient audit trail queries by patient and date range.
CREATE INDEX IF NOT EXISTS idx_audit_log_hn ON wf_audit_log(hn);
CREATE INDEX IF NOT EXISTS idx_audit_log_timestamp ON wf_audit_log(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_log_action ON wf_audit_log(action);
