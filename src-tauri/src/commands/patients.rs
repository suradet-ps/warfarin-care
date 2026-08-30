//! Patient management commands - enroll, list, detail, status change.

use chrono::Utc;
use tauri::State;

use warfarin_core::{
  dose::calculator::calculate_ttr,
  models::{
    inr::InrRecord,
    patient::{ActivePatientSummary, EnrollmentInput, HosxpPatient, PatientDetail, WfPatient},
  },
};
use warfarin_db::{
  mysql::{get_dispensing_history, get_hosxp_patient},
  sqlite::{
    AppState, enroll_patient as db_enroll, get_active_patients as db_get_active,
    get_inr_from_visits, get_latest_visit_dates_by_hns, get_latest_visit_dose_by_hns,
    get_patient_by_hn, get_pending_appointments, get_visit_history,
    update_patient_status as db_update_status,
  },
};

#[tauri::command]
pub async fn get_active_patients(state: State<'_, AppState>) -> Result<Vec<WfPatient>, String> {
  state.require_auth().await?;
  db_get_active(&state.pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_active_patient_summaries(
  state: State<'_, AppState>,
) -> Result<Vec<ActivePatientSummary>, String> {
  state.require_auth().await?;
  let patients = db_get_active(&state.pool)
    .await
    .map_err(|e| e.to_string())?;
  let hns: Vec<String> = patients.iter().map(|patient| patient.hn.clone()).collect();
  let config_result = crate::commands::settings::get_mysql_config_internal(&state.pool)
    .await
    .map_err(|e| e.clone())?;

  let (hosxp_map, mysql_inr_map) = if let Some(config) = config_result {
    warfarin_db::mysql::get_dashboard_patient_data(&config, &hns)
      .await
      .unwrap_or_else(|_| {
        (
          std::collections::HashMap::new(),
          std::collections::HashMap::new(),
        )
      })
  } else {
    (
      std::collections::HashMap::new(),
      std::collections::HashMap::new(),
    )
  };

  let appointments = get_pending_appointments(&state.pool)
    .await
    .unwrap_or_default();
  let latest_dose_by_hn = get_latest_visit_dose_by_hns(&state.pool, &hns)
    .await
    .unwrap_or_default();
  let latest_visit_date_by_hn = get_latest_visit_dates_by_hns(&state.pool, &hns)
    .await
    .unwrap_or_default();
  let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();

  let mut summaries = Vec::with_capacity(patients.len());
  for patient in patients {
    let inr_records = match mysql_inr_map.get(&patient.hn) {
      Some(records) if !records.is_empty() => records.clone(),
      _ => get_inr_from_visits(&state.pool, &patient.hn)
        .await
        .unwrap_or_default(),
    };
    let latest_inr = inr_records.last().cloned();
    let inr_pairs: Vec<(String, f64)> = inr_records
      .iter()
      .map(|record| (record.date.clone(), record.value))
      .collect();
    let ttr6months = calculate_ttr(
      &inr_pairs,
      patient.target_inr_low,
      patient.target_inr_high,
      182,
    );
    let current_dose_mgday = latest_dose_by_hn.get(&patient.hn).copied().flatten();
    let next_appointment = find_next_appointment(&appointments, &patient.hn, &today);
    let last_visit_date = latest_visit_date_by_hn.get(&patient.hn).cloned();

    summaries.push(ActivePatientSummary {
      hosxp_info: hosxp_map
        .get(&patient.hn)
        .cloned()
        .unwrap_or_else(|| HosxpPatient {
          hn: patient.hn.clone(),
          pname: String::new(),
          fname: format!("HN {}", patient.hn),
          lname: "(ไม่พบข้อมูล HOSxP)".to_string(),
          birthday: String::new(),
          sex: "U".to_string(),
          addrpart: None,
          phone: None,
        }),
      patient,
      latest_inr,
      current_dose_mgday,
      ttr6months,
      next_appointment,
      last_visit_date,
    });
  }

  Ok(summaries)
}

#[tauri::command]
pub async fn enroll_patient(
  input: EnrollmentInput,
  state: State<'_, AppState>,
) -> Result<i64, String> {
  state.require_auth().await?;
  db_enroll(&state.pool, &input, &state.machine_id)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_patient_detail(
  hn: String,
  state: State<'_, AppState>,
) -> Result<PatientDetail, String> {
  state.require_auth().await?;
  let patient = get_patient_by_hn(&state.pool, &hn)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("patient not found: {hn}"))?;

  // Fetch HOSxP demographics - graceful fallback if MySQL is unavailable.
  let hosxp_info = try_get_hosxp_patient(&state, &hn).await;

  // Combine INR from clinic visits (SQLite) as fallback / supplement.
  let inr_records = get_inr_records(&state, &hn).await;
  let dispensing_history = try_get_dispensing_history(&state, &hn).await;

  let latest_inr = inr_records
    .iter()
    .max_by(|a, b| a.date.cmp(&b.date))
    .cloned();

  // Latest confirmed new dose from most recent visit.
  let visits = get_visit_history(&state.pool, &hn)
    .await
    .unwrap_or_default();
  let current_dose = visits.first().and_then(|v| v.new_dose_mgday);

  // TTR over last 6 months (182 days).
  let inr_pairs: Vec<(String, f64)> = inr_records
    .iter()
    .map(|r| (r.date.clone(), r.value))
    .collect();
  let ttr6 = calculate_ttr(
    &inr_pairs,
    patient.target_inr_low,
    patient.target_inr_high,
    182,
  );

  // Next scheduled appointment.
  let appointments = get_pending_appointments(&state.pool)
    .await
    .unwrap_or_default();
  let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();
  let next_appt = find_next_appointment(&appointments, &hn, &today);

  Ok(PatientDetail {
    patient,
    hosxp_info,
    latest_inr,
    current_dose_mgday: current_dose,
    ttr6months: ttr6,
    next_appointment: next_appt,
    inr_history: inr_records,
    dispensing_history,
  })
}

#[tauri::command]
pub async fn update_patient_status(
  hn: String,
  status: String,
  reason: String,
  effective_date: Option<String>,
  state: State<'_, AppState>,
) -> Result<(), String> {
  state.require_auth().await?;
  db_update_status(
    &state.pool,
    &hn,
    &status,
    Some(reason.as_str()),
    effective_date.as_deref(),
    &state.machine_id,
  )
  .await
  .map_err(|e| e.to_string())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Attempts to fetch patient demographics from `HOSxP` `MySQL`, returning a
/// placeholder on any failure so the UI is never blocked.
async fn try_get_hosxp_patient(state: &AppState, hn: &str) -> HosxpPatient {
  let config_result = crate::commands::settings::get_mysql_config_internal(&state.pool)
    .await
    .ok()
    .flatten();

  if let Some(config) = config_result
    && let Ok(Some(info)) = get_hosxp_patient(&config, hn).await
  {
    return info;
  }

  HosxpPatient {
    hn: hn.to_string(),
    pname: String::new(),
    fname: format!("HN {hn}"),
    lname: "(ไม่พบข้อมูล HOSxP)".to_string(),
    birthday: String::new(),
    sex: "U".to_string(),
    addrpart: None,
    phone: None,
  }
}

async fn try_get_dispensing_history(
  state: &AppState,
  hn: &str,
) -> Vec<warfarin_core::models::dispensing::DispensingRecord> {
  let config_result = crate::commands::settings::get_mysql_config_internal(&state.pool)
    .await
    .ok()
    .flatten();

  if let Some(config) = config_result
    && let Ok(records) = get_dispensing_history(&config, hn).await
  {
    return records;
  }

  Vec::new()
}

/// Returns INR records, preferring `HOSxP` `MySQL` (dual-source merge) and
/// falling back to clinic-recorded INR values from `wf_visits`.
pub(crate) async fn get_inr_records_by_hns(
  state: &AppState,
  hns: &[String],
) -> std::collections::HashMap<String, Vec<InrRecord>> {
  if hns.is_empty() {
    return std::collections::HashMap::new();
  }

  let config_result = crate::commands::settings::get_mysql_config_internal(&state.pool)
    .await
    .ok()
    .flatten();

  let mut records_by_hn = if let Some(config) = config_result {
    warfarin_db::mysql::get_dashboard_patient_data(&config, hns)
      .await
      .map(|(_, inr_map)| inr_map)
      .unwrap_or_default()
  } else {
    std::collections::HashMap::new()
  };

  for hn in hns {
    if records_by_hn.contains_key(hn) {
      continue;
    }

    let fallback_records = get_inr_from_visits(&state.pool, hn)
      .await
      .unwrap_or_default();
    if !fallback_records.is_empty() {
      records_by_hn.insert(hn.clone(), fallback_records);
    }
  }

  records_by_hn
}

pub(crate) async fn get_inr_records(state: &AppState, hn: &str) -> Vec<InrRecord> {
  let hns = vec![hn.to_string()];
  let mut records_by_hn = get_inr_records_by_hns(state, &hns).await;
  records_by_hn.remove(hn).unwrap_or_default()
}

fn find_next_appointment(
  appointments: &[warfarin_core::models::appointment::WfAppointment],
  hn: &str,
  today: &str,
) -> Option<String> {
  appointments
    .iter()
    .find(|appointment| appointment.hn == hn && appointment.appt_date.as_str() >= today)
    .map(|appointment| appointment.appt_date.clone())
}
