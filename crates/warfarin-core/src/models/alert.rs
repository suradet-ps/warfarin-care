use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientAlert {
  pub hn: String,
  pub patient_name: String,
  pub alert_type: String,
  pub severity: String,
  pub message: String,
  pub value: Option<f64>,
  pub date: Option<String>,
}
