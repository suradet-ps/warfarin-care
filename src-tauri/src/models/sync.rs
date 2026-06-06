use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WfPatientSync {
  pub sync_id: Option<String>,
  pub machine_id: Option<String>,
  pub hn: String,
  pub enrolled_at: String,
  pub enrolled_by: Option<String>,
  pub status: String,
  pub indication: Option<String>,
  pub target_inr_low: f64,
  pub target_inr_high: f64,
  pub notes: Option<String>,
  pub created_at: String,
  pub updated_at: String,
  pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WfVisitSync {
  pub sync_id: Option<String>,
  pub machine_id: Option<String>,
  pub hn: String,
  pub visit_date: String,
  pub inr_value: Option<f64>,
  pub inr_source: Option<String>,
  pub current_dose_mgday: Option<f64>,
  pub dose_detail: Option<String>,
  pub new_dose_mgday: Option<f64>,
  pub new_dose_detail: Option<String>,
  pub new_dose_description: Option<String>,
  pub selected_dose_option: Option<String>,
  pub dose_changed: i64,
  pub next_appointment: Option<String>,
  pub next_inr_due: Option<String>,
  pub physician: Option<String>,
  pub notes: Option<String>,
  pub side_effects: Option<String>,
  pub adherence: Option<String>,
  pub created_by: Option<String>,
  pub reviewed_at: Option<String>,
  pub reviewed_by: Option<String>,
  pub created_at: String,
  pub updated_at: String,
  pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WfDoseHistorySync {
  pub sync_id: Option<String>,
  pub machine_id: Option<String>,
  pub hn: String,
  pub changed_at: String,
  pub old_dose_mgday: Option<f64>,
  pub new_dose_mgday: Option<f64>,
  pub old_detail: Option<String>,
  pub new_detail: Option<String>,
  pub reason: Option<String>,
  pub inr_at_change: Option<f64>,
  pub changed_by: Option<String>,
  pub created_at: String,
  pub updated_at: String,
  pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WfAppointmentSync {
  pub sync_id: Option<String>,
  pub machine_id: Option<String>,
  pub hn: String,
  pub appt_date: String,
  pub appt_type: Option<String>,
  pub status: String,
  pub notes: Option<String>,
  pub source_visit_id: Option<i64>,
  pub source_visit_sync_id: Option<String>,
  pub generated_from_visit: i64,
  pub created_at: String,
  pub updated_at: String,
  pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WfOutcomeSync {
  pub sync_id: Option<String>,
  pub machine_id: Option<String>,
  pub hn: String,
  pub event_date: String,
  pub event_type: String,
  pub description: Option<String>,
  pub inr_at_event: Option<f64>,
  pub action_taken: Option<String>,
  pub created_by: Option<String>,
  pub created_at: String,
  pub updated_at: String,
  pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WfPatientStatusHistorySync {
  pub sync_id: Option<String>,
  pub machine_id: Option<String>,
  pub hn: String,
  pub status: String,
  pub reason: Option<String>,
  pub effective_date: String,
  pub created_at: String,
  pub updated_at: String,
  pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
  pub pushed: usize,
  pub pulled: usize,
  pub conflicts: usize,
  pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
  pub pending_count: i64,
  pub last_sync_at: Option<String>,
  pub configured: bool,
  pub machine_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncSummary {
  pub has_anon_key: bool,
  pub supabase_url: Option<String>,
}

/// Common interface implemented by every row type the sync layer pulls from
/// Supabase. Lets the generic `pull_table` helper in `commands::sync` read
/// the row's stable identifier and last-write timestamp without knowing the
/// concrete shape of each table.
pub trait PulledRow {
  fn sync_id(&self) -> Option<&String>;
  fn updated_at(&self) -> &str;
}

impl PulledRow for WfPatientSync {
  fn sync_id(&self) -> Option<&String> { self.sync_id.as_ref() }
  fn updated_at(&self) -> &str { &self.updated_at }
}

impl PulledRow for WfVisitSync {
  fn sync_id(&self) -> Option<&String> { self.sync_id.as_ref() }
  fn updated_at(&self) -> &str { &self.updated_at }
}

impl PulledRow for WfDoseHistorySync {
  fn sync_id(&self) -> Option<&String> { self.sync_id.as_ref() }
  fn updated_at(&self) -> &str { &self.updated_at }
}

impl PulledRow for WfAppointmentSync {
  fn sync_id(&self) -> Option<&String> { self.sync_id.as_ref() }
  fn updated_at(&self) -> &str { &self.updated_at }
}

impl PulledRow for WfOutcomeSync {
  fn sync_id(&self) -> Option<&String> { self.sync_id.as_ref() }
  fn updated_at(&self) -> &str { &self.updated_at }
}

impl PulledRow for WfPatientStatusHistorySync {
  fn sync_id(&self) -> Option<&String> { self.sync_id.as_ref() }
  fn updated_at(&self) -> &str { &self.updated_at }
}
