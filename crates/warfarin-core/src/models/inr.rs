use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InrRecord {
  pub date: String,
  pub value: f64,
  pub source: String,
  pub lab_order_number: Option<String>,
  pub vn: Option<String>,
}
