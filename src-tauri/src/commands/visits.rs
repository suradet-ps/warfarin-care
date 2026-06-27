//! Visit management commands — save, query, and dose suggestion.

use tauri::State;

use warfarin_core::{
  dose::calculator::suggest_dose_from_daily,
  models::visit::{DoseSuggestion, VisitInput, WfVisit},
};
use warfarin_db::sqlite::{
  AppState, approve_visit as db_approve_visit, delete_visit as db_delete_visit,
  get_pending_review_count as db_pending_count, get_pending_review_visits as db_pending,
  get_visit_by_id as db_get_visit_by_id, get_visit_history as db_history, save_visit as db_save,
  update_visit as db_update_visit,
};

#[tauri::command]
pub async fn get_visit_history(
  hn: String,
  state: State<'_, AppState>,
) -> Result<Vec<WfVisit>, String> {
  db_history(&state.pool, &hn)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_visit_by_id(visit_id: i64, state: State<'_, AppState>) -> Result<WfVisit, String> {
  db_get_visit_by_id(&state.pool, visit_id)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("visit not found: {visit_id}"))
}

#[tauri::command]
pub async fn save_visit(visit: VisitInput, state: State<'_, AppState>) -> Result<i64, String> {
  db_save(&state.pool, &visit, &state.machine_id)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_visit(
  visit_id: i64,
  visit: VisitInput,
  state: State<'_, AppState>,
) -> Result<(), String> {
  db_update_visit(&state.pool, visit_id, &visit, &state.machine_id)
    .await
    .map_err(|e| e.to_string())
}

/// Computes a warfarin dose adjustment suggestion from daily-dose inputs.
///
/// Validation, mg/day→mg/week conversion, and the calculator call live in
/// `warfarin_core::dose::calculator::suggest_dose_from_daily`; this command
/// is a thin IPC wrapper that surfaces errors as strings for the frontend.
#[tauri::command]
pub async fn suggest_dose(
  current_dose_mgday: f64,
  current_inr: f64,
  target_low: f64,
  target_high: f64,
) -> Result<DoseSuggestion, String> {
  suggest_dose_from_daily(current_dose_mgday, current_inr, target_low, target_high)
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_visit(visit_id: i64, state: State<'_, AppState>) -> Result<(), String> {
  db_delete_visit(&state.pool, visit_id, &state.machine_id)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_pending_review_visits(state: State<'_, AppState>) -> Result<Vec<WfVisit>, String> {
  db_pending(&state.pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_pending_review_count(state: State<'_, AppState>) -> Result<i64, String> {
  db_pending_count(&state.pool)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn approve_visit(
  visit_id: i64,
  reviewer: String,
  state: State<'_, AppState>,
) -> Result<(), String> {
  db_approve_visit(&state.pool, visit_id, &reviewer, &state.machine_id)
    .await
    .map_err(|e| e.to_string())
}
