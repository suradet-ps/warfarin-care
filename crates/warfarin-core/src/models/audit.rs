use serde::{Deserialize, Serialize};

/// A single entry in the unified audit trail.
///
/// Records every clinical action: visit saves, dose changes, status
/// changes, adverse events, and authentication events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogEntry {
  pub id: i64,
  pub hn: Option<String>,
  pub action: String,
  pub actor: String,
  pub timestamp: String,
  pub old_value: Option<String>,
  pub new_value: Option<String>,
  pub detail: Option<String>,
  pub created_at: String,
}

/// Input for creating a new audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogInput {
  pub hn: Option<String>,
  pub action: String,
  pub actor: String,
  pub old_value: Option<String>,
  pub new_value: Option<String>,
  pub detail: Option<String>,
}

/// Filter criteria for querying the audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogFilter {
  pub hn: Option<String>,
  pub action: Option<String>,
  pub date_from: Option<String>,
  pub date_to: Option<String>,
  pub page: Option<u32>,
  pub page_size: Option<u32>,
}

/// Standard audit action constants.
pub const ACTION_VISIT_SAVED: &str = "visit_saved";
pub const ACTION_VISIT_UPDATED: &str = "visit_updated";
pub const ACTION_VISIT_DELETED: &str = "visit_deleted";
pub const ACTION_DOSE_CHANGED: &str = "dose_changed";
pub const ACTION_STATUS_CHANGED: &str = "status_changed";
pub const ACTION_ADVERSE_EVENT: &str = "adverse_event";
pub const ACTION_LOGIN: &str = "login";
pub const ACTION_LOGOUT: &str = "logout";
pub const ACTION_PATIENT_ENROLLED: &str = "patient_enrolled";
pub const ACTION_INTERACTION_CHECK: &str = "interaction_check";
