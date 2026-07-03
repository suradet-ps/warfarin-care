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
  /// `true` only when the appointment is in the past, the clinic ran on that
  /// day, AND the patient has no `wf_visits` row for that day. Populated by
  /// `get_pending_appointments`; older callers leave it `None` so the field
  /// stays backward-compatible over Tauri IPC.
  #[serde(skip_serializing_if = "Option::is_none", default)]
  pub is_overdue: Option<bool>,
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
