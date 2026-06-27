use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WfOutcome {
  pub id: i64,
  pub hn: String,
  pub event_date: String,
  pub event_type: String,
  pub description: Option<String>,
  pub inr_at_event: Option<f64>,
  pub action_taken: Option<String>,
  pub created_by: Option<String>,
  pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutcomeInput {
  pub hn: String,
  pub event_date: String,
  pub event_type: String,
  pub description: Option<String>,
  pub inr_at_event: Option<f64>,
  pub action_taken: Option<String>,
  pub created_by: Option<String>,
}
