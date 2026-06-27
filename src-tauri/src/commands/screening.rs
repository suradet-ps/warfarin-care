//! Screening command — queries `HOSxP` warfarin dispensing records.
//!
//! When `MySQL` is not configured this falls back to an empty response so the UI
//! remains functional.  The `HOSxP` `MySQL` integration is in `warfarin_db::mysql`.

use tauri::State;

use warfarin_core::{models::patient::SearchResponse, screening::normalize_search_filters};
use warfarin_db::{
  mysql::search_hosxp_warfarin_patients,
  sqlite::{AppState, get_all_enrolled_hns},
};

#[tauri::command]
pub async fn search_warfarin_patients(
  filters: warfarin_core::models::patient::SearchFilters,
  state: State<'_, AppState>,
) -> Result<SearchResponse, String> {
  // Clamp inputs to safe ranges before the SQL layer sees them. Returning
  // silently-clamped values keeps the FE working (e.g. if it asks for 500).
  let filters = normalize_search_filters(filters);

  // Get all enrolled HNs from SQLite to flag patients already in the clinic.
  let enrolled_hns = get_all_enrolled_hns(&state.pool)
    .await
    .map_err(|e| e.to_string())?;

  let config = crate::commands::settings::get_mysql_config_internal(&state.pool)
    .await
    .map_err(|e| e.clone())?
    .ok_or_else(|| "ยังไม่ได้ตั้งค่าการเชื่อมต่อ HOSxP".to_string())?;

  search_hosxp_warfarin_patients(&config, &filters, &enrolled_hns)
    .await
    .map_err(|e| format!("failed to search HOSxP warfarin patients: {e:#}"))
}
