//! Alert engine command - evaluates all active patients for clinical alerts.

use chrono::{NaiveDate, Utc};
use tauri::State;

use crate::commands::patients::get_inr_records_by_hns;
use warfarin_core::{dose::calculator::calculate_ttr, models::alert::PatientAlert};
use warfarin_db::sqlite::{AppState, get_active_patients, get_pending_appointments};

const TTR_RED_THRESHOLD: f64 = 50.0;
const TTR_YELLOW_THRESHOLD: f64 = 65.0;
const INR_CRITICAL_HIGH: f64 = 4.0;
const INR_CRITICAL_LOW: f64 = 1.5;
const NO_INR_DAYS: i64 = 90;

#[tauri::command]
pub async fn get_patient_alerts(state: State<'_, AppState>) -> Result<Vec<PatientAlert>, String> {
  state.require_auth().await?;
  let patients = get_active_patients(&state.pool)
    .await
    .map_err(|e| e.to_string())?;

  let pending_appts = get_pending_appointments(&state.pool)
    .await
    .unwrap_or_default();
  let hns: Vec<String> = patients.iter().map(|patient| patient.hn.clone()).collect();
  let inr_records_by_hn = get_inr_records_by_hns(&state, &hns).await;

  let today = Utc::now().date_naive();
  let mut alerts: Vec<PatientAlert> = Vec::new();

  for patient in &patients {
    let inr_records = inr_records_by_hn
      .get(&patient.hn)
      .cloned()
      .unwrap_or_default();

    let latest_inr = inr_records
      .iter()
      .max_by(|a, b| a.date.cmp(&b.date))
      .cloned();

    let display_name = format!("HN {}", patient.hn);

    // ── INR-based alerts ───────────────────────────────────────────────
    if let Some(ref rec) = latest_inr {
      let inr = rec.value;

      if inr > 5.0 {
        alerts.push(PatientAlert {
          hn: patient.hn.clone(),
          patient_name: display_name.clone(),
          alert_type: "critical_high_inr".to_string(),
          severity: "critical".to_string(),
          message: format!("INR สูงวิกฤต {inr:.1} - เสี่ยงเลือดออกรุนแรง"),
          value: Some(inr),
          date: Some(rec.date.clone()),
        });
      } else if inr > INR_CRITICAL_HIGH {
        alerts.push(PatientAlert {
          hn: patient.hn.clone(),
          patient_name: display_name.clone(),
          alert_type: "critical_high_inr".to_string(),
          severity: "critical".to_string(),
          message: format!("INR สูงวิกฤต {inr:.1} (> 4.0)"),
          value: Some(inr),
          date: Some(rec.date.clone()),
        });
      } else if inr < INR_CRITICAL_LOW {
        alerts.push(PatientAlert {
          hn: patient.hn.clone(),
          patient_name: display_name.clone(),
          alert_type: "critical_low_inr".to_string(),
          severity: "critical".to_string(),
          message: format!("INR ต่ำวิกฤต {inr:.1} (< 1.5) - เสี่ยงลิ่มเลือด"),
          value: Some(inr),
          date: Some(rec.date.clone()),
        });
      } else if inr > patient.target_inr_high {
        alerts.push(PatientAlert {
          hn: patient.hn.clone(),
          patient_name: display_name.clone(),
          alert_type: "inr_above_range".to_string(),
          severity: "warning".to_string(),
          message: format!(
            "INR {inr:.1} สูงกว่าเป้าหมาย ({:.1}-{:.1})",
            patient.target_inr_low, patient.target_inr_high
          ),
          value: Some(inr),
          date: Some(rec.date.clone()),
        });
      } else if inr < patient.target_inr_low {
        alerts.push(PatientAlert {
          hn: patient.hn.clone(),
          patient_name: display_name.clone(),
          alert_type: "inr_below_range".to_string(),
          severity: "warning".to_string(),
          message: format!(
            "INR {inr:.1} ต่ำกว่าเป้าหมาย ({:.1}-{:.1})",
            patient.target_inr_low, patient.target_inr_high
          ),
          value: Some(inr),
          date: Some(rec.date.clone()),
        });
      }

      // ── No recent INR check ────────────────────────────────────────
      if let Ok(last_date) = NaiveDate::parse_from_str(&rec.date, "%Y-%m-%d") {
        let days_ago = (today - last_date).num_days();
        if days_ago > NO_INR_DAYS {
          alerts.push(PatientAlert {
            hn: patient.hn.clone(),
            patient_name: display_name.clone(),
            alert_type: "no_recent_inr".to_string(),
            severity: "warning".to_string(),
            message: format!("ไม่ได้ตรวจ INR มา {days_ago} วัน"),
            value: None,
            date: Some(rec.date.clone()),
          });
        }
      }
    } else {
      alerts.push(PatientAlert {
        hn: patient.hn.clone(),
        patient_name: display_name.clone(),
        alert_type: "no_recent_inr".to_string(),
        severity: "warning".to_string(),
        message: "ยังไม่มีข้อมูล INR".to_string(),
        value: None,
        date: None,
      });
    }

    // ── TTR alert ──────────────────────────────────────────────────────
    let inr_pairs: Vec<(String, f64)> = inr_records
      .iter()
      .map(|r| (r.date.clone(), r.value))
      .collect();
    if let Some(ttr) = calculate_ttr(
      &inr_pairs,
      patient.target_inr_low,
      patient.target_inr_high,
      182,
    ) {
      if ttr < TTR_RED_THRESHOLD {
        alerts.push(PatientAlert {
          hn: patient.hn.clone(),
          patient_name: display_name.clone(),
          alert_type: "low_ttr".to_string(),
          severity: "critical".to_string(),
          message: format!("TTR ต่ำมาก {ttr:.0}% (เกณฑ์ ≥ 65%)"),
          value: Some(ttr),
          date: None,
        });
      } else if ttr < TTR_YELLOW_THRESHOLD {
        alerts.push(PatientAlert {
          hn: patient.hn.clone(),
          patient_name: display_name.clone(),
          alert_type: "low_ttr".to_string(),
          severity: "warning".to_string(),
          message: format!("TTR ต่ำ {ttr:.0}% (เกณฑ์ ≥ 65%)"),
          value: Some(ttr),
          date: None,
        });
      }
    }

    // ── Missed appointment ─────────────────────────────────────────────
    // Only emit a missed-appointment alert when the appointment is truly
    // overdue: the clinic actually ran on that day AND the patient has no
    // visit record for that day. Pure past dates on which the clinic was
    // closed do not count (e.g. holidays, off-days). The
    // `is_overdue` flag is pre-computed in `get_pending_appointments`.
    for appt in pending_appts
      .iter()
      .filter(|a| a.hn == patient.hn && a.is_overdue == Some(true))
    {
      if let Ok(appt_date) = NaiveDate::parse_from_str(&appt.appt_date, "%Y-%m-%d")
        && appt_date < today
      {
        let days_overdue = (today - appt_date).num_days();
        alerts.push(PatientAlert {
          hn: patient.hn.clone(),
          patient_name: display_name.clone(),
          alert_type: "missed_appointment".to_string(),
          severity: "warning".to_string(),
          message: format!("ขาดนัด {days_overdue} วัน (นัด {})", appt.appt_date),
          value: None,
          date: Some(appt.appt_date.clone()),
        });
      }
    }
  }

  // Sort: critical first, then by HN.
  alerts.sort_by(|a, b| {
    let severity_order = |s: &str| i32::from(s != "critical");
    severity_order(&a.severity)
      .cmp(&severity_order(&b.severity))
      .then(a.hn.cmp(&b.hn))
  });

  Ok(alerts)
}
