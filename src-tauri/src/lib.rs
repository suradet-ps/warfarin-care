//! Tauri application shell for warfarin-care.
//!
//! This crate is a thin wrapper: it registers Tauri plugins, wires up
//! `AppState`, and exposes the 46 `#[tauri::command]` handlers that delegate
//! to `warfarin-core` (pure logic) and `warfarin-db` (data access). No
//! business logic lives here.
//!
//! # Error convention
//!
//! Every `#[tauri::command]` returns `Result<T, String>`: internal errors
//! from `anyhow`/`sqlx` are mapped to a human-readable string at the command
//! boundary so the Vue frontend receives a displayable message. That
//! contract is uniform, so `clippy::missing_errors_doc` is relaxed at the
//! crate level rather than duplicated across 46 handlers.

#![warn(clippy::pedantic)]
// Tauri commands uniformly return Result<_, String> for the frontend; see
// the crate docs above.
#![allow(clippy::missing_errors_doc)]
// Several command handlers orchestrate multiple data sources (HOSxP MySQL
// + local SQLite + INR/TTR aggregation) and are inherently longer than 100
// lines. Splitting them would scatter the IPC-visible flow; the long-form
// is preferred until a dedicated orchestration layer is introduced.
#![allow(clippy::too_many_lines)]

pub mod commands;

use anyhow::{Context, Result};
use commands::{
  alerts::get_patient_alerts,
  appointments::{
    get_appointment_day_load, get_appointments, get_pending_appointments, schedule_appointment,
  },
  auth::{current_user, has_users, is_logged_in, login, logout, setup_admin},
  inr::{get_inr_history, get_latest_inr},
  interaction::{
    add_drug_interaction, delete_drug_interaction, get_all_drug_interactions,
    get_patient_drug_interactions, search_hosxp_drugs,
  },
  outcomes::{get_outcomes, record_adverse_event},
  patients::{
    enroll_patient, get_active_patient_summaries, get_active_patients, get_patient_detail,
    update_patient_status,
  },
  reports::{calculate_clinic_ttr, calculate_ttr, get_report_data},
  screening::search_warfarin_patients,
  settings::{
    get_mysql_config_for_ui, get_mysql_config_internal, get_mysql_config_status, get_setting_value,
    get_settings, save_mysql_config, save_setting, test_mysql_connection,
  },
  slip::save_slip_pdf,
  sync::{
    get_sync_status, get_sync_summary, pull_from_supabase, push_to_supabase, save_supabase_config,
    test_supabase_connection,
  },
  visits::{
    approve_visit, delete_visit, get_pending_review_count, get_pending_review_visits,
    get_visit_by_id, get_visit_history, save_visit, suggest_dose, update_visit,
  },
};
use sqlx::mysql::MySqlPoolOptions;
use tauri::{App, Emitter, Manager};
use warfarin_db::sqlite::{AppState, init_pool};

fn initialise_app_state(app: &mut App) -> Result<()> {
  let app_handle = app.handle().clone();
  let machine_id =
    commands::sync::get_or_create_machine_id(&app_handle).map_err(anyhow::Error::msg)?;

  let app_dir = app_handle
    .path()
    .app_data_dir()
    .context("failed to resolve app data directory")?;

  std::fs::create_dir_all(&app_dir).with_context(|| {
    format!(
      "failed to create app data directory at {}",
      app_dir.display()
    )
  })?;

  let db_path = app_dir.join("warfarin.db");
  let pool = tauri::async_runtime::block_on(init_pool(db_path.clone())).with_context(|| {
    format!(
      "failed to initialise SQLite database at {}",
      db_path.display()
    )
  })?;

  app_handle.manage(AppState::new(pool.clone(), machine_id));

  let app_handle_clone = app_handle.clone();
  let pool_clone = pool.clone();
  tauri::async_runtime::spawn(async move {
    let _ = app_handle_clone.emit("splash-status", "กำลังโหลดฐานข้อมูล...");

    match get_mysql_config_internal(&pool_clone).await {
      Ok(Some(config)) => {
        let _ = app_handle_clone.emit("splash-status", "กำลังเชื่อมต่อ MySQL...");

        let url = format!(
          "mysql://{}:{}@{}:{}/{}",
          config.username, config.password, config.host, config.port, config.database
        );

        let pool_result = tokio::time::timeout(
          std::time::Duration::from_secs(8),
          MySqlPoolOptions::new().max_connections(5).connect(&url),
        )
        .await;

        match pool_result {
          Ok(Ok(_pool)) => {
            println!(
              "[warfarin] Auto-connected to MySQL ({}:{})",
              config.host, config.port
            );
            let _ = app_handle_clone.emit("splash-status", "เชื่อมต่อสำเร็จ ✓");
          }
          Ok(Err(e)) => {
            eprintln!("[warfarin] Auto-connect to MySQL failed: {e}");
            let _ = app_handle_clone.emit("splash-status", "เชื่อมต่อล้มเหลว (ใช้งานออฟไลน์ได้)");
          }
          Err(_) => {
            eprintln!("[warfarin] MySQL auto-connect timed out after 8 s");
            let _ = app_handle_clone.emit("splash-status", "เชื่อมต่อหมดเวลา (ใช้งานออฟไลน์ได้)");
          }
        }
      }
      Ok(None) => {
        let _ = app_handle_clone.emit("splash-status", "พร้อมใช้งาน (ยังไม่ตั้งค่า MySQL)");
      }
      Err(e) => {
        eprintln!("[warfarin] Failed to load saved DB config: {e}");
        let _ = app_handle_clone.emit("splash-status", "โหลดการตั้งค่าล้มเหลว");
      }
    }

    // Keep splash visible for minimum 2.5s (handled by Vue)
    tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
  });

  Ok(())
}

pub fn run() -> tauri::Result<()> {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_store::Builder::new().build())
    .setup(|app| initialise_app_state(app).map_err(Into::into))
    .invoke_handler(tauri::generate_handler![
      search_warfarin_patients,
      get_active_patients,
      get_active_patient_summaries,
      get_patient_detail,
      enroll_patient,
      update_patient_status,
      get_appointments,
      get_pending_appointments,
      get_appointment_day_load,
      get_visit_history,
      get_visit_by_id,
      save_visit,
      update_visit,
      delete_visit,
      suggest_dose,
      get_patient_alerts,
      get_outcomes,
      record_adverse_event,
      get_settings,
      save_setting,
      get_setting_value,
      test_mysql_connection,
      save_mysql_config,
      get_mysql_config_for_ui,
      get_mysql_config_status,
      get_inr_history,
      get_latest_inr,
      schedule_appointment,
      calculate_ttr,
      calculate_clinic_ttr,
      get_report_data,
      get_all_drug_interactions,
      add_drug_interaction,
      delete_drug_interaction,
      search_hosxp_drugs,
      get_patient_drug_interactions,
      save_slip_pdf,
      get_pending_review_visits,
      get_pending_review_count,
      approve_visit,
      save_supabase_config,
      test_supabase_connection,
      push_to_supabase,
      pull_from_supabase,
      get_sync_status,
      get_sync_summary,
      has_users,
      setup_admin,
      login,
      logout,
      is_logged_in,
      current_user,
    ])
    .run(tauri::generate_context!())
}
