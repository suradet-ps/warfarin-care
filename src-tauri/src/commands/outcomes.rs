use tauri::State;

use warfarin_core::models::outcome::{OutcomeInput, WfOutcome};
use warfarin_db::sqlite::{
  AppState, get_outcomes as db_get_outcomes, record_adverse_event as db_record_outcome,
};

#[tauri::command]
pub async fn get_outcomes(
  hn: String,
  state: State<'_, AppState>,
) -> Result<Vec<WfOutcome>, String> {
  state.require_auth().await?;
  db_get_outcomes(&state.pool, &hn)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn record_adverse_event(
  event: OutcomeInput,
  state: State<'_, AppState>,
) -> Result<i64, String> {
  state.require_auth().await?;
  db_record_outcome(&state.pool, &event, &state.machine_id)
    .await
    .map_err(|e| e.to_string())
}
