use tauri::State;

use crate::db::sqlite::{
  AppState, get_outcomes as db_get_outcomes, record_adverse_event as db_record_outcome,
};
use warfarin_core::models::outcome::{OutcomeInput, WfOutcome};

#[tauri::command]
pub async fn get_outcomes(
  hn: String,
  state: State<'_, AppState>,
) -> Result<Vec<WfOutcome>, String> {
  db_get_outcomes(&state.pool, &hn)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn record_adverse_event(
  event: OutcomeInput,
  state: State<'_, AppState>,
) -> Result<i64, String> {
  db_record_outcome(&state.pool, &event, &state.machine_id)
    .await
    .map_err(|e| e.to_string())
}
