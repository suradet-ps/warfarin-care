use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WfAppointment {
  pub id: i64,
  pub hn: String,
  pub appt_date: String,
  pub appt_type: Option<String>,
  pub status: String,
  pub notes: Option<String>,
  pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppointmentInput {
  pub hn: String,
  pub appt_date: String,
  pub appt_type: Option<String>,
  pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppointmentDayLoad {
  pub appt_date: String,
  pub scheduled_count: i64,
}
