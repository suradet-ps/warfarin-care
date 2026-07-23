use serde::{Deserialize, Serialize};

/// Severity level for a drug interaction with warfarin.
///
/// Contraindicated interactions block the visit save. Major interactions
/// require dose adjustment. Moderate interactions require monitoring.
/// Minor interactions are informational only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
  Contraindicated,
  Major,
  Moderate,
  Minor,
}

impl Severity {
  /// Returns the Thai display label for this severity level.
  #[must_use]
  pub fn label_thai(&self) -> &'static str {
    match self {
      Self::Contraindicated => "ห้ามใช้ร่วม",
      Self::Major => "หลีกเลี่ยง",
      Self::Moderate => "ระวัง",
      Self::Minor => "ทราบ",
    }
  }

  /// Returns the English display label for this severity level.
  #[must_use]
  pub fn label_english(&self) -> &'static str {
    match self {
      Self::Contraindicated => "Contraindicated",
      Self::Major => "Major",
      Self::Moderate => "Moderate",
      Self::Minor => "Minor",
    }
  }
}

impl std::fmt::Display for Severity {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let s = match self {
      Self::Contraindicated => "contraindicated",
      Self::Major => "major",
      Self::Moderate => "moderate",
      Self::Minor => "minor",
    };
    f.write_str(s)
  }
}

impl std::str::FromStr for Severity {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_lowercase().as_str() {
      "contraindicated" => Ok(Self::Contraindicated),
      "major" => Ok(Self::Major),
      "moderate" => Ok(Self::Moderate),
      "minor" => Ok(Self::Minor),
      _ => Err(format!("unknown severity: {s}")),
    }
  }
}

/// A full persisted drug interaction record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrugInteraction {
  pub id: i64,
  pub icode: String,
  pub drug_name: String,
  pub strength: Option<String>,
  pub interaction_type: String,
  pub severity: String,
  pub clinical_effect: Option<String>,
  pub management: Option<String>,
  pub evidence_level: Option<String>,
  pub created_at: String,
  pub updated_at: String,
}

/// Input/creation DTO for a drug interaction (no id or timestamps).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrugInteractionInput {
  pub icode: String,
  pub drug_name: String,
  pub strength: Option<String>,
  pub interaction_type: String,
  pub severity: String,
  pub clinical_effect: Option<String>,
  pub management: Option<String>,
  pub evidence_level: Option<String>,
}

pub type InteractionType = &'static str;
pub const INTERACTION_INCREASE: InteractionType = "increase";
pub const INTERACTION_DECREASE: InteractionType = "decrease";

pub const INTERACTION_TYPES: [InteractionType; 2] = [INTERACTION_INCREASE, INTERACTION_DECREASE];

/// A medication currently prescribed to a patient.
///
/// Used as input to the interaction checker. Only the fields needed
/// for matching are included — no I/O or runtime coupling.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientMedication {
  pub icode: String,
  pub drug_name: String,
  pub strength: Option<String>,
}

/// A detected interaction between a patient's medication and warfarin.
///
/// Returned by [`crate::interaction::check`]. Contains all information
/// needed for the frontend to render severity badges and clinical guidance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Interaction {
  pub icode: String,
  pub drug_name: String,
  pub strength: Option<String>,
  pub interaction_type: String,
  pub severity: Severity,
  pub clinical_effect: Option<String>,
  pub management: Option<String>,
  pub evidence_level: Option<String>,
}
