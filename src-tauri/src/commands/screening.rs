//! Screening command — queries HOSxP warfarin dispensing records.
//!
//! When MySQL is not configured this falls back to an empty response so the UI
//! remains functional.  The HOSxP MySQL integration is in `db::mysql`.

use tauri::State;

use crate::db::{
  mysql::search_hosxp_warfarin_patients,
  sqlite::{AppState, get_all_enrolled_hns},
};
use warfarin_core::models::patient::{SearchFilters, SearchResponse};

/// Hard cap on `page_size` regardless of what the FE sends. Stops a malicious
/// or buggy caller from asking the MySQL host for tens of thousands of rows
/// in a single round trip. 200 is well above the practical row count a
/// clinician can scan on a single page.
const MAX_PAGE_SIZE: u32 = 200;
const MAX_KEYWORD_LEN: usize = 200;

#[tauri::command]
pub async fn search_warfarin_patients(
  mut filters: SearchFilters,
  state: State<'_, AppState>,
) -> Result<SearchResponse, String> {
  // Clamp inputs to safe ranges before the SQL layer sees them. Returning
  // silently-clamped values keeps the FE working (e.g. if it asks for 500).
  if filters.page_size == 0 {
    filters.page_size = 50;
  }
  if filters.page_size > MAX_PAGE_SIZE {
    filters.page_size = MAX_PAGE_SIZE;
  }
  if filters.page == 0 {
    filters.page = 1;
  }
  if let Some(keyword) = filters.keyword.as_deref() {
    let trimmed = keyword.trim();
    filters.keyword = if trimmed.is_empty() {
      None
    } else if trimmed.chars().count() > MAX_KEYWORD_LEN {
      Some(trimmed.chars().take(MAX_KEYWORD_LEN).collect::<String>())
    } else {
      Some(trimmed.to_string())
    };
  }

  // Get all enrolled HNs from SQLite to flag patients already in the clinic.
  let enrolled_hns = get_all_enrolled_hns(&state.pool)
    .await
    .map_err(|e| e.to_string())?;

  let config = crate::commands::settings::get_mysql_config_internal(&state.pool)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "ยังไม่ได้ตั้งค่าการเชื่อมต่อ HOSxP".to_string())?;

  search_hosxp_warfarin_patients(&config, &filters, &enrolled_hns)
    .await
    .map_err(|e| format!("failed to search HOSxP warfarin patients: {:#}", e))
}
