//! Visit management commands - save, query, and dose suggestion.

use tauri::State;

use warfarin_core::{
  dose::calculator::suggest_dose_from_daily,
  models::{
    audit::{ACTION_VISIT_DELETED, ACTION_VISIT_SAVED, ACTION_VISIT_UPDATED, AuditLogInput},
    auth::Permission,
    visit::{DoseSuggestion, VisitInput, WfVisit},
  },
};
use warfarin_db::sqlite::{
  AppState, approve_visit as db_approve_visit, delete_visit as db_delete_visit,
  get_pending_review_count as db_pending_count, get_pending_review_visits as db_pending,
  get_visit_by_id as db_get_visit_by_id, get_visit_history as db_history,
  insert_audit_log as db_insert_audit, save_visit as db_save, update_visit as db_update_visit,
};

#[tauri::command]
pub async fn get_visit_history(
  hn: String,
  state: State<'_, AppState>,
) -> Result<Vec<WfVisit>, String> {
  state.require_auth().await?;
  db_history(&state.pool, &hn)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_visit_by_id(visit_id: i64, state: State<'_, AppState>) -> Result<WfVisit, String> {
  state.require_auth().await?;
  db_get_visit_by_id(&state.pool, visit_id)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("visit not found: {visit_id}"))
}

#[tauri::command]
pub async fn save_visit(mut visit: VisitInput, state: State<'_, AppState>) -> Result<i64, String> {
  let user = state.require_permission(Permission::WriteVisit).await?;
  visit.created_by = Some(user.username.clone());
  let visit_id = db_save(&state.pool, &visit, &state.machine_id)
    .await
    .map_err(|e| e.to_string())?;

  let detail = serde_json::json!({
    "visit_id": visit_id,
    "inr": visit.inr_value,
    "dose_change": visit.dose_changed,
  });
  let _ = db_insert_audit(
    &state.pool,
    &AuditLogInput {
      hn: Some(visit.hn.clone()),
      action: ACTION_VISIT_SAVED.to_string(),
      actor: user.username.clone(),
      old_value: None,
      new_value: visit.new_dose_mgday.map(|d| format!("{d} mg/day")),
      detail: Some(detail.to_string()),
    },
  )
  .await;

  Ok(visit_id)
}

#[tauri::command]
pub async fn update_visit(
  visit_id: i64,
  mut visit: VisitInput,
  state: State<'_, AppState>,
) -> Result<(), String> {
  let user = state.require_permission(Permission::WriteVisit).await?;
  visit.created_by = Some(user.username.clone());
  db_update_visit(&state.pool, visit_id, &visit, &state.machine_id)
    .await
    .map_err(|e| e.to_string())?;

  let detail = serde_json::json!({
    "visit_id": visit_id,
    "inr": visit.inr_value,
    "dose_change": visit.dose_changed,
  });
  let _ = db_insert_audit(
    &state.pool,
    &AuditLogInput {
      hn: Some(visit.hn.clone()),
      action: ACTION_VISIT_UPDATED.to_string(),
      actor: user.username.clone(),
      old_value: None,
      new_value: visit.new_dose_mgday.map(|d| format!("{d} mg/day")),
      detail: Some(detail.to_string()),
    },
  )
  .await;

  Ok(())
}

/// Computes a warfarin dose adjustment suggestion from daily-dose inputs.
///
/// Validation, mg/day->mg/week conversion, and the calculator call live in
/// `warfarin_core::dose::calculator::suggest_dose_from_daily`; this command
/// is a thin IPC wrapper that surfaces errors as strings for the frontend.
#[tauri::command]
pub async fn suggest_dose(
  current_dose_mgday: f64,
  current_inr: f64,
  target_low: f64,
  target_high: f64,
  state: State<'_, AppState>,
) -> Result<DoseSuggestion, String> {
  state.require_auth().await?;
  suggest_dose_from_daily(current_dose_mgday, current_inr, target_low, target_high)
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_visit(visit_id: i64, state: State<'_, AppState>) -> Result<(), String> {
  let user = state.require_permission(Permission::WriteVisit).await?;
  let visit = db_get_visit_by_id(&state.pool, visit_id)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("visit not found: {visit_id}"))?;

  db_delete_visit(&state.pool, visit_id, &state.machine_id)
    .await
    .map_err(|e| e.to_string())?;

  let _ = db_insert_audit(
    &state.pool,
    &AuditLogInput {
      hn: Some(visit.hn),
      action: ACTION_VISIT_DELETED.to_string(),
      actor: user.username,
      old_value: visit.new_dose_mgday.map(|d| format!("{d} mg/day")),
      new_value: None,
      detail: Some(serde_json::json!({ "visit_id": visit_id }).to_string()),
    },
  )
  .await;

  Ok(())
}

#[tauri::command]
pub async fn get_pending_review_visits(state: State<'_, AppState>) -> Result<Vec<WfVisit>, String> {
  state.require_auth().await?;
  db_pending(&state.pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_pending_review_count(state: State<'_, AppState>) -> Result<i64, String> {
  state.require_auth().await?;
  db_pending_count(&state.pool)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn approve_visit(visit_id: i64, state: State<'_, AppState>) -> Result<(), String> {
  let user = state.require_permission(Permission::ApproveVisit).await?;
  db_approve_visit(&state.pool, visit_id, &user.username, &state.machine_id)
    .await
    .map_err(|e| e.to_string())
}
