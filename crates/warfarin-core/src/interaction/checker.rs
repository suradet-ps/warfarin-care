//! Pure interaction-checking logic.
//!
//! Given a list of patient medications and a list of known interaction rules
//! (loaded from `wf_drug_interactions`), returns all matching interactions
//! sorted by severity (most severe first).

use std::str::FromStr;

use crate::models::interaction::{DrugInteraction, Interaction, PatientMedication, Severity};

/// Checks a patient's medications against known warfarin interaction rules.
///
/// Returns a `Vec<Interaction>` sorted by severity (contraindicated first),
/// then alphabetically by drug name. Each interaction includes clinical
/// effect, management advice, and evidence level when available.
///
/// # Arguments
///
/// * `patient_medications` — drugs currently prescribed to the patient.
/// * `interaction_rules` — the full set of known warfarin interaction
///   definitions (loaded from `wf_drug_interactions`).
///
/// # Examples
///
/// ```rust
/// use warfarin_core::interaction::check;
/// use warfarin_core::models::interaction::{PatientMedication, DrugInteraction, Severity};
///
/// let meds = vec![
///     PatientMedication {
///         icode: "123".into(),
///         drug_name: "Amiodarone".into(),
///         strength: Some("200mg".into()),
///     },
/// ];
/// let rules = vec![
///     DrugInteraction {
///         id: 1,
///         icode: "123".into(),
///         drug_name: "Amiodarone".into(),
///         strength: Some("200mg".into()),
///         interaction_type: "increase".into(),
///         severity: "major".into(),
///         clinical_effect: Some("Increases warfarin effect".into()),
///         management: Some("Reduce warfarin dose 30-50%".into()),
///         evidence_level: Some("A".into()),
///         created_at: "2024-01-01".into(),
///         updated_at: "2024-01-01".into(),
///     },
/// ];
/// let result = check(&meds, &rules);
/// assert_eq!(result.len(), 1);
/// assert_eq!(result[0].severity, Severity::Major);
/// ```
#[must_use]
pub fn check(
  patient_medications: &[PatientMedication],
  interaction_rules: &[DrugInteraction],
) -> Vec<Interaction> {
  if patient_medications.is_empty() || interaction_rules.is_empty() {
    return Vec::new();
  }

  // Build a lookup map from icode -> DrugInteraction for O(1) matching.
  let mut rules_by_icode: std::collections::HashMap<&str, &DrugInteraction> =
    std::collections::HashMap::with_capacity(interaction_rules.len());
  for rule in interaction_rules {
    rules_by_icode.insert(&rule.icode, rule);
  }

  let mut interactions: Vec<Interaction> = Vec::new();

  for med in patient_medications {
    if let Some(rule) = rules_by_icode.get(med.icode.as_str()) {
      let severity = Severity::from_str(&rule.severity).unwrap_or(Severity::Minor);
      interactions.push(Interaction {
        icode: med.icode.clone(),
        drug_name: med.drug_name.clone(),
        strength: med.strength.clone(),
        interaction_type: rule.interaction_type.clone(),
        severity,
        clinical_effect: rule.clinical_effect.clone(),
        management: rule.management.clone(),
        evidence_level: rule.evidence_level.clone(),
      });
    }
  }

  // Sort by severity (most severe first), then by drug name.
  interactions.sort_by(|a, b| {
    severity_order(b.severity)
      .cmp(&severity_order(a.severity))
      .then_with(|| a.drug_name.cmp(&b.drug_name))
  });

  interactions
}

/// Returns a numeric sort key for severity ordering (higher = more severe).
fn severity_order(s: Severity) -> u8 {
  match s {
    Severity::Contraindicated => 4,
    Severity::Major => 3,
    Severity::Moderate => 2,
    Severity::Minor => 1,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn make_med(icode: &str, name: &str) -> PatientMedication {
    PatientMedication {
      icode: icode.into(),
      drug_name: name.into(),
      strength: None,
    }
  }

  fn make_rule(icode: &str, name: &str, severity: &str, interaction_type: &str) -> DrugInteraction {
    DrugInteraction {
      id: 1,
      icode: icode.into(),
      drug_name: name.into(),
      strength: None,
      interaction_type: interaction_type.into(),
      severity: severity.into(),
      clinical_effect: Some("test effect".into()),
      management: Some("test management".into()),
      evidence_level: Some("A".into()),
      created_at: "2024-01-01".into(),
      updated_at: "2024-01-01".into(),
    }
  }

  #[test]
  fn check_empty_medications_returns_empty() {
    let result = check(&[], &[make_rule("1", "Drug", "major", "increase")]);
    assert!(result.is_empty());
  }

  #[test]
  fn check_empty_rules_returns_empty() {
    let result = check(&[make_med("1", "Drug")], &[]);
    assert!(result.is_empty());
  }

  #[test]
  fn check_matching_medication_returns_interaction() {
    let meds = vec![make_med("123", "Amiodarone")];
    let rules = vec![make_rule("123", "Amiodarone", "major", "increase")];
    let result = check(&meds, &rules);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].icode, "123");
    assert_eq!(result[0].severity, Severity::Major);
    assert_eq!(result[0].interaction_type, "increase");
  }

  #[test]
  fn check_non_matching_medication_returns_empty() {
    let meds = vec![make_med("999", "Paracetamol")];
    let rules = vec![make_rule("123", "Amiodarone", "major", "increase")];
    let result = check(&meds, &rules);
    assert!(result.is_empty());
  }

  #[test]
  fn check_multiple_medications_some_match() {
    let meds = vec![
      make_med("123", "Amiodarone"),
      make_med("456", "Ibuprofen"),
      make_med("789", "Paracetamol"),
    ];
    let rules = vec![
      make_rule("123", "Amiodarone", "major", "increase"),
      make_rule("456", "Ibuprofen", "moderate", "increase"),
    ];
    let result = check(&meds, &rules);
    assert_eq!(result.len(), 2);
  }

  #[test]
  fn check_results_sorted_by_severity_desc() {
    let meds = vec![
      make_med("111", "MinorDrug"),
      make_med("222", "MajorDrug"),
      make_med("333", "ContraDrug"),
      make_med("444", "ModerateDrug"),
    ];
    let rules = vec![
      make_rule("111", "MinorDrug", "minor", "increase"),
      make_rule("222", "MajorDrug", "major", "increase"),
      make_rule("333", "ContraDrug", "contraindicated", "increase"),
      make_rule("444", "ModerateDrug", "moderate", "increase"),
    ];
    let result = check(&meds, &rules);
    assert_eq!(result.len(), 4);
    assert_eq!(result[0].severity, Severity::Contraindicated);
    assert_eq!(result[1].severity, Severity::Major);
    assert_eq!(result[2].severity, Severity::Moderate);
    assert_eq!(result[3].severity, Severity::Minor);
  }

  #[test]
  fn check_same_severity_sorted_alphabetically() {
    let meds = vec![make_med("222", "ZebraDrug"), make_med("111", "AlphaDrug")];
    let rules = vec![
      make_rule("222", "ZebraDrug", "major", "increase"),
      make_rule("111", "AlphaDrug", "major", "decrease"),
    ];
    let result = check(&meds, &rules);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].drug_name, "AlphaDrug");
    assert_eq!(result[1].drug_name, "ZebraDrug");
  }

  #[test]
  fn check_preserves_clinical_effect_and_management() {
    let meds = vec![PatientMedication {
      icode: "123".into(),
      drug_name: "Amiodarone".into(),
      strength: Some("200mg".into()),
    }];
    let rules = vec![DrugInteraction {
      id: 1,
      icode: "123".into(),
      drug_name: "Amiodarone".into(),
      strength: Some("200mg".into()),
      interaction_type: "increase".into(),
      severity: "major".into(),
      clinical_effect: Some("Increases warfarin effect via CYP2C9 inhibition".into()),
      management: Some("Reduce warfarin dose by 30-50% and monitor INR closely".into()),
      evidence_level: Some("A".into()),
      created_at: "2024-01-01".into(),
      updated_at: "2024-01-01".into(),
    }];
    let result = check(&meds, &rules);
    assert_eq!(result.len(), 1);
    assert_eq!(
      result[0].clinical_effect.as_deref(),
      Some("Increases warfarin effect via CYP2C9 inhibition")
    );
    assert_eq!(
      result[0].management.as_deref(),
      Some("Reduce warfarin dose by 30-50% and monitor INR closely")
    );
    assert_eq!(result[0].evidence_level.as_deref(), Some("A"));
    assert_eq!(result[0].strength.as_deref(), Some("200mg"));
  }

  #[test]
  fn check_unknown_severity_defaults_to_minor() {
    let meds = vec![make_med("123", "Drug")];
    let rules = vec![DrugInteraction {
      id: 1,
      icode: "123".into(),
      drug_name: "Drug".into(),
      strength: None,
      interaction_type: "increase".into(),
      severity: "unknown_value".into(),
      clinical_effect: None,
      management: None,
      evidence_level: None,
      created_at: "2024-01-01".into(),
      updated_at: "2024-01-01".into(),
    }];
    let result = check(&meds, &rules);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].severity, Severity::Minor);
  }

  #[test]
  fn check_duplicate_icode_in_meds_matches_once() {
    // A patient cannot have duplicate icode medications in practice,
    // but the function should handle it gracefully.
    let meds = vec![make_med("123", "Amiodarone"), make_med("123", "Amiodarone")];
    let rules = vec![make_rule("123", "Amiodarone", "major", "increase")];
    let result = check(&meds, &rules);
    // Both medications match the rule — both produce an interaction.
    assert_eq!(result.len(), 2);
  }

  #[test]
  fn severity_display_roundtrip() {
    assert_eq!(Severity::Contraindicated.to_string(), "contraindicated");
    assert_eq!("major".parse::<Severity>().unwrap(), Severity::Major);
    assert_eq!("MODERATE".parse::<Severity>().unwrap(), Severity::Moderate);
    assert!("unknown".parse::<Severity>().is_err());
  }

  #[test]
  fn severity_label_thai() {
    assert_eq!(Severity::Contraindicated.label_thai(), "ห้ามใช้ร่วม");
    assert_eq!(Severity::Major.label_thai(), "หลีกเลี่ยง");
    assert_eq!(Severity::Moderate.label_thai(), "ระวัง");
    assert_eq!(Severity::Minor.label_thai(), "ทราบ");
  }
}
