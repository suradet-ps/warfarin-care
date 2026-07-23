//! Audit trail commands — log and query clinical actions.

use tauri::State;

use warfarin_core::models::audit::{AuditLogEntry, AuditLogFilter, AuditLogInput};
use warfarin_db::sqlite::{
  AppState, get_audit_log as db_get_audit_log, get_patient_audit_log as db_get_patient_audit_log,
  insert_audit_log as db_insert_audit_log,
};

#[tauri::command]
pub async fn insert_audit_log(
  input: AuditLogInput,
  state: State<'_, AppState>,
) -> Result<i64, String> {
  state.require_auth().await?;
  db_insert_audit_log(&state.pool, &input)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_audit_log(
  filter: AuditLogFilter,
  state: State<'_, AppState>,
) -> Result<Vec<AuditLogEntry>, String> {
  state.require_auth().await?;
  db_get_audit_log(&state.pool, &filter)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_patient_audit_log(
  hn: String,
  limit: u32,
  state: State<'_, AppState>,
) -> Result<Vec<AuditLogEntry>, String> {
  state.require_auth().await?;
  db_get_patient_audit_log(&state.pool, &hn, limit)
    .await
    .map_err(|e| e.to_string())
}
