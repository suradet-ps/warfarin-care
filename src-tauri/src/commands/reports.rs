//! TTR report command.

use serde_json::{Value, json};
use tauri::State;

use crate::{
  commands::patients::{get_inr_records, get_inr_records_by_hns},
  db::sqlite::{AppState, get_active_patients, get_outcome_counts_by_hns},
  dose::calculator::calculate_ttr as calc_ttr,
};

/// Calculates TTR (Rosendaal method) for a single patient over a given window.
///
/// `window_days` — 0 means all-time.
#[tauri::command]
pub async fn calculate_ttr(
  hn: String,
  window_days: u32,
  state: State<'_, AppState>,
) -> Result<Option<f64>, String> {
  let patient = crate::db::sqlite::get_patient_by_hn(&state.pool, &hn)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("patient not found: {hn}"))?;

  let inr_records = get_inr_records(&state, &hn).await;
  let pairs: Vec<(String, f64)> = inr_records
    .iter()
    .map(|r| (r.date.clone(), r.value))
    .collect();

  let window = if window_days == 0 {
    u32::MAX
  } else {
    window_days
  };
  Ok(calc_ttr(
    &pairs,
    patient.target_inr_low,
    patient.target_inr_high,
    window,
  ))
}

/// Calculates mean TTR across all active patients (for clinic-level report).
#[tauri::command]
pub async fn calculate_clinic_ttr(
  window_days: u32,
  state: State<'_, AppState>,
) -> Result<f64, String> {
  let patients = get_active_patients(&state.pool)
    .await
    .map_err(|e| e.to_string())?;

  let window = if window_days == 0 {
    u32::MAX
  } else {
    window_days
  };
  let hns: Vec<String> = patients.iter().map(|patient| patient.hn.clone()).collect();
  let inr_records_by_hn = get_inr_records_by_hns(&state, &hns).await;
  let mut total = 0.0f64;
  let mut count = 0usize;

  for patient in &patients {
    let inr_records = inr_records_by_hn
      .get(&patient.hn)
      .cloned()
      .unwrap_or_default();
    let pairs: Vec<(String, f64)> = inr_records
      .iter()
      .map(|r| (r.date.clone(), r.value))
      .collect();
    if let Some(ttr) = calc_ttr(
      &pairs,
      patient.target_inr_low,
      patient.target_inr_high,
      window,
    ) {
      total += ttr;
      count += 1;
    }
  }

  if count == 0 {
    Ok(0.0)
  } else {
    Ok(total / count as f64)
  }
}

#[tauri::command]
pub async fn get_report_data(
  report_type: String,
  state: State<'_, AppState>,
) -> Result<Value, String> {
  match report_type.as_str() {
    "census" => {
      let patients = crate::db::sqlite::get_all_patients(&state.pool)
        .await
        .map_err(|e| e.to_string())?;

      let mut active = 0usize;
      let mut inactive = 0usize;
      let mut transferred = 0usize;
      let mut discharged = 0usize;
      let mut deceased = 0usize;

      for patient in &patients {
        match patient.status.as_str() {
          "active" => active += 1,
          "inactive" => inactive += 1,
          "transferred" => transferred += 1,
          "discharged" => discharged += 1,
          "deceased" => deceased += 1,
          _ => {}
        }
      }

      Ok(json!({
        "active": active,
        "inactive": inactive,
        "transferred": transferred,
        "discharged": discharged,
        "deceased": deceased,
        "total": patients.len(),
      }))
    }
    "ttr" => {
      let mean_ttr = calculate_clinic_ttr(182, state).await?;
      Ok(json!({ "meanTtr": mean_ttr }))
    }
    "adverse" => {
      let patients = get_active_patients(&state.pool)
        .await
        .map_err(|e| e.to_string())?;
      let hns: Vec<String> = patients.iter().map(|patient| patient.hn.clone()).collect();
      let outcome_counts = get_outcome_counts_by_hns(&state.pool, &hns)
        .await
        .map_err(|e| e.to_string())?;
      let total_events: usize = outcome_counts.values().map(|count| *count as usize).sum();

      Ok(json!({ "totalEvents": total_events }))
    }
    "inr_distribution" => {
      // Histogram of all INR values across active patients, bucketed by
      // clinical convention (<1.5, 1.5-2.0, 2.0-3.0, 3.0-4.0, >4.0).
      // Bounds are: half-open `[low, high)` so a value of 2.0 lands in
      // "in_range" not "below_range".
      let patients = get_active_patients(&state.pool)
        .await
        .map_err(|e| e.to_string())?;
      let hns: Vec<String> = patients.iter().map(|patient| patient.hn.clone()).collect();
      let inr_records_by_hn = get_inr_records_by_hns(&state, &hns).await;

      let mut below_1_5 = 0usize;
      let mut between_1_5_2_0 = 0usize;
      let mut between_2_0_3_0 = 0usize;
      let mut between_3_0_4_0 = 0usize;
      let mut above_4_0 = 0usize;

      for records in inr_records_by_hn.values() {
        for rec in records {
          if rec.value < 1.5 {
            below_1_5 += 1;
          } else if rec.value < 2.0 {
            between_1_5_2_0 += 1;
          } else if rec.value < 3.0 {
            between_2_0_3_0 += 1;
          } else if rec.value < 4.0 {
            between_3_0_4_0 += 1;
          } else {
            above_4_0 += 1;
          }
        }
      }
      Ok(json!({
        "lt_1_5": below_1_5,
        "1_5_to_2_0": between_1_5_2_0,
        "2_0_to_3_0": between_2_0_3_0,
        "3_0_to_4_0": between_3_0_4_0,
        "gt_4_0": above_4_0,
        "total": below_1_5 + between_1_5_2_0 + between_2_0_3_0 + between_3_0_4_0 + above_4_0,
      }))
    }
    "missed_appointments" => {
      // Patients with at least one scheduled appointment whose appt_date
      // is in the past and whose status has not been updated.
      let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT hn, appt_date FROM wf_appointments \
          WHERE status = 'scheduled' AND appt_date < date('now') \
          ORDER BY appt_date DESC LIMIT 200",
      )
      .fetch_all(&state.pool)
      .await
      .map_err(|e| e.to_string())?;
      let total: usize = rows.len();
      Ok(json!({
        "total": total,
        "items": rows
          .into_iter()
          .map(|(hn, appt_date)| json!({ "hn": hn, "apptDate": appt_date }))
          .collect::<Vec<_>>(),
      }))
    }
    "dose_adjustment_frequency" => {
      // For each active patient, count how many of their visits had
      // dose_changed = 1, then bucket by frequency.
      let rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT v.hn, SUM(CASE WHEN v.dose_changed = 1 THEN 1 ELSE 0 END) AS changes \
           FROM wf_visits v \
          WHERE v.deleted_at IS NULL \
          GROUP BY v.hn",
      )
      .fetch_all(&state.pool)
      .await
      .map_err(|e| e.to_string())?;
      let total_visits: i64 =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM wf_visits WHERE deleted_at IS NULL")
          .fetch_one(&state.pool)
          .await
          .map_err(|e| e.to_string())?;
      let total_changes: i64 = rows.iter().map(|(_, c)| *c).sum();
      let patients_with_changes = rows.iter().filter(|(_, c)| *c > 0).count();
      let change_ratio = if total_visits > 0 {
        total_changes as f64 / total_visits as f64
      } else {
        0.0
      };
      Ok(json!({
        "totalChanges": total_changes,
        "totalVisits": total_visits,
        "patientsWithChanges": patients_with_changes,
        "changeRatio": change_ratio,
      }))
    }
    "monthly_cohort" => {
      // New enrollments per month for the last 12 months.
      let rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT strftime('%Y-%m', enrolled_at) AS month, COUNT(*) AS count \
           FROM wf_patients \
          WHERE enrolled_at >= date('now', '-12 months') \
          GROUP BY month \
          ORDER BY month ASC",
      )
      .fetch_all(&state.pool)
      .await
      .map_err(|e| e.to_string())?;
      Ok(json!({
        "items": rows
          .into_iter()
          .map(|(month, count)| json!({ "month": month, "count": count }))
          .collect::<Vec<_>>(),
      }))
    }
    _ => Err(format!("unsupported report type: {report_type}")),
  }
}
