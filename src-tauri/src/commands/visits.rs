//! Visit management commands — save, query, and dose suggestion.

use tauri::State;

use crate::{
  db::sqlite::{
    AppState, approve_visit as db_approve_visit, delete_visit as db_delete_visit,
    get_pending_review_count as db_pending_count, get_pending_review_visits as db_pending,
    get_visit_by_id as db_get_visit_by_id, get_visit_history as db_history, save_visit as db_save,
    update_visit as db_update_visit,
  },
  dose::calculator::suggest_dose as suggest_dose_impl,
  models::visit::{DoseSuggestion, VisitInput, WfVisit},
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

/// Hard cap on the dose magnitude the calculator is willing to consider.
/// Anything beyond this is almost certainly a data-entry error (max
/// practical warfarin dose is around 15 mg/day).
const MAX_DOSE_MGDAY: f64 = 20.0;
/// Hard cap on the INR value. Real-world INR > 10 is rare and indicates
/// a critical situation; we don't compute dose adjustments beyond this.
const MAX_INR: f64 = 10.0;

#[tauri::command]
pub async fn suggest_dose(
  current_dose_mgday: f64,
  current_inr: f64,
  target_low: f64,
  target_high: f64,
) -> Result<DoseSuggestion, String> {
  // Reject obviously bad input before the calculator sees it.
  if !current_dose_mgday.is_finite() || !(0.0..=MAX_DOSE_MGDAY).contains(&current_dose_mgday) {
    return Err(format!(
      "current_dose_mgday must be 0..={MAX_DOSE_MGDAY}, got {current_dose_mgday}"
    ));
  }
  if !current_inr.is_finite() || !(0.5..=MAX_INR).contains(&current_inr) {
    return Err(format!(
      "current_inr must be 0.5..={MAX_INR}, got {current_inr}"
    ));
  }
  if !target_low.is_finite() || !target_high.is_finite() {
    return Err("target_low and target_high must be finite".to_string());
  }
  // The calculator works in mg/week; convert from mg/day at the boundary so
  // the frontend never has to think in weekly units.
  let current_dose_mgweek = current_dose_mgday * 7.0;
  Ok(suggest_dose_impl(
    current_dose_mgweek,
    current_inr,
    target_low,
    target_high,
  ))
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
