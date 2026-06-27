use serde::{Deserialize, Serialize};

use super::visit::DoseSchedule;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedDoseInfo {
  pub tablets_per_dose: f64,
  pub mg_per_dose: f64,
  pub mg_per_week: f64,
  pub mg_per_day_average: f64,
  pub schedule: DoseSchedule,
  pub matched_days: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DispensingRecord {
  pub hn: String,
  pub vn: Option<String>,
  pub an: Option<String>,
  pub vstdate: String,
  pub icode: String,
  pub drug_name: String,
  pub strength: String,
  pub qty: f64,
  pub unitprice: f64,
  pub drugusage_code: Option<String>,
  pub sp_use_code: Option<String>,
  pub usage_text: Option<String>,
  pub parsed_dose: Option<ParsedDoseInfo>,
  pub usage_parse_note: Option<String>,
}
