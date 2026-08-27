//! INR history commands - fetches from `HOSxP` `MySQL` (dual source) with
//! fallback to clinic-recorded values in `wf_visits`.

use tauri::State;

use crate::commands::patients::get_inr_records;
use warfarin_core::models::inr::InrRecord;
use warfarin_db::sqlite::AppState;

#[tauri::command]
pub async fn get_inr_history(
  hn: String,
  state: State<'_, AppState>,
) -> Result<Vec<InrRecord>, String> {
  state.require_auth().await?;
  let mut records = get_inr_records(&state, &hn).await;
  records.sort_by(|a, b| a.date.cmp(&b.date));
  Ok(records)
}

#[tauri::command]
pub async fn get_latest_inr(
  hn: String,
  state: State<'_, AppState>,
) -> Result<Option<InrRecord>, String> {
  state.require_auth().await?;
  let mut records = get_inr_records(&state, &hn).await;
  records.sort_by(|a, b| a.date.cmp(&b.date));
  Ok(records.into_iter().last())
}
