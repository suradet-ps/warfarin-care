//! Appointment scheduling command.

use tauri::State;

use warfarin_core::models::appointment::{AppointmentDayLoad, AppointmentInput, WfAppointment};
use warfarin_db::sqlite::{
  AppState, get_appointment_day_load as db_get_appointment_day_load,
  get_appointments as db_get_appointments, get_pending_appointments as db_get_pending_appointments,
  schedule_appointment as db_schedule,
};

#[tauri::command]
pub async fn get_appointments(
  hn: String,
  state: State<'_, AppState>,
) -> Result<Vec<WfAppointment>, String> {
  state.require_auth().await?;
  db_get_appointments(&state.pool, &hn)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn schedule_appointment(
  appt: AppointmentInput,
  state: State<'_, AppState>,
) -> Result<i64, String> {
  state.require_auth().await?;
  db_schedule(&state.pool, &appt, &state.machine_id)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_pending_appointments(
  state: State<'_, AppState>,
) -> Result<Vec<WfAppointment>, String> {
  state.require_auth().await?;
  db_get_pending_appointments(&state.pool)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_appointment_day_load(
  appt_date: String,
  state: State<'_, AppState>,
) -> Result<AppointmentDayLoad, String> {
  state.require_auth().await?;
  db_get_appointment_day_load(&state.pool, &appt_date)
    .await
    .map_err(|e| e.to_string())
}
