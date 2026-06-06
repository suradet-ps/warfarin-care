//! SQLite persistence layer for the warfarin clinic application.
//!
//! Uses runtime queries (`sqlx::query()`) throughout so no DATABASE_URL is
//! needed at compile time. All public functions return `anyhow::Result`.

use anyhow::{Context, Result, bail};
use chrono::{Datelike, Utc};
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool, Transaction, sqlite::SqlitePoolOptions};
use std::path::PathBuf;
use uuid::Uuid;

use crate::models::{
  appointment::{AppointmentDayLoad, AppointmentInput, WfAppointment},
  inr::InrRecord,
  interaction::{DrugInteraction, DrugInteractionInput},
  outcome::{OutcomeInput, WfOutcome},
  patient::{EnrollmentInput, WfPatient},
  visit::{DoseSchedule, RegimenOptionSnapshot, TotalPillsSummary, VisitInput, WfVisit},
};

/// Decompose a single-day dose (in mg) into a list of pill pieces.
/// Each piece is `(strength, is_half)` — `is_half = false` means a whole
/// pill, `true` means half a pill of that strength. The algorithm searches
/// the small combination space (whole/half of 2/3/5 mg) for the combination
/// that (a) sums to the prescribed dose in 0.5 mg steps, and (b) uses the
/// fewest total pieces.
///
/// Pill strengths follow AGENTS.md (2 mg, 3 mg, 5 mg). Any dose that is
/// not a multiple of 0.5 mg (e.g. 4.7 mg) is rounded to the nearest 0.5.
/// Doses that are still unrepresentable with these pieces (e.g. 0.5 mg
/// exactly — there's no 1 mg pill to half) are skipped.
fn pills_for_dose(dose_mg: f64) -> Vec<(u8, bool)> {
  if dose_mg <= 0.0 {
    return vec![];
  }
  let half_units = (dose_mg * 2.0).round() as i32;
  if half_units <= 0 {
    return vec![];
  }
  // Each entry in this table is the minimum-piece split for a given
  // half_units value (0..=30 → 0..=15 mg). A piece is one of:
  //   (5, false)=1×5mg, (5, true)=½×5mg, (3, false), (3, true), (2, false), (2, true)
  // We pre-compute the optimal split for the common warfarin dose range.
  // The search uses the simple observation: at most 3 of any one strength
  // are needed for doses up to 15 mg.
  let strengths = [5u8, 3, 2];
  let mut best: Option<Vec<(u8, bool)>> = None;
  for w5 in 0..=3 {
    for w3 in 0..=5 {
      for w2 in 0..=7 {
        for h5 in 0..=1 {
          for h3 in 0..=1 {
            for h2 in 0..=1 {
              let total_half_units = 2 * (5 * w5 + 3 * w3 + 2 * w2)
                + (5 * h5 + 3 * h3 + 2 * h2);
              if total_half_units != half_units {
                continue;
              }
              // Reject combinations that use the same strength in whole
              // AND half form on the same day (pharmacist can't practically
              // do that without two different pill bottles).
              if h5 == 1 && w5 > 0 {
                continue;
              }
              if h3 == 1 && w3 > 0 {
                continue;
              }
              if h2 == 1 && w2 > 0 {
                continue;
              }
              let pieces: Vec<(u8, bool)> = strengths
                .iter()
                .flat_map(|s| std::iter::repeat_n(*s, w_count(*s, w5, w3, w2)))
                .map(|s| (s, false))
                .chain(
                  strengths
                    .iter()
                    .flat_map(|s| std::iter::repeat_n(*s, h_count(*s, h5, h3, h2)))
                    .map(|s| (s, true)),
                )
                .collect();
              let count = pieces.len();
              if best.as_ref().is_none_or(|b| count < b.len()) {
                best = Some(pieces);
              }
            }
          }
        }
      }
    }
  }
  best.unwrap_or_default()
}

fn w_count(strength: u8, w5: i32, w3: i32, w2: i32) -> usize {
  match strength {
    5 => w5 as usize,
    3 => w3 as usize,
    2 => w2 as usize,
    _ => 0,
  }
}

fn h_count(strength: u8, h5: i32, h3: i32, h2: i32) -> usize {
  match strength {
    5 => h5 as usize,
    3 => h3 as usize,
    2 => h2 as usize,
    _ => 0,
  }
}

/// Pure pill-tally: given a list of per-day doses in mg, return the total
/// (whole, half) count per pill strength. Pure function — no I/O — so the
/// caller (`calculate_pills_summary`) handles the date range and the unit
/// tests can target the algorithm directly.
fn tally_pills(day_doses: &[f64]) -> std::collections::HashMap<u8, (u32, u32)> {
  let mut pill_counts: std::collections::HashMap<u8, (u32, u32)> = std::collections::HashMap::new();
  for &dose in day_doses {
    for (strength, is_half) in pills_for_dose(dose) {
      let entry = pill_counts.entry(strength).or_insert((0, 0));
      if is_half {
        entry.1 += 1;
      } else {
        entry.0 += 1;
      }
    }
  }
  pill_counts
}

fn calculate_pills_summary(
  visit_date: &str,
  new_dose_detail: &DoseSchedule,
  next_appointment: &str,
) -> Option<crate::models::visit::TotalPillsSummary> {
  let visit = chrono::NaiveDate::parse_from_str(visit_date, "%Y-%m-%d").ok()?;
  let next = chrono::NaiveDate::parse_from_str(next_appointment, "%Y-%m-%d").ok()?;

  let days = (next - visit).num_days();
  if days <= 0 {
    return None;
  }

  // Build the per-day dose sequence for the whole dispense period. The
  // schedule is a 7-day template; the sequence repeats it for `days`.
  let weekly = [
    new_dose_detail.mon,
    new_dose_detail.tue,
    new_dose_detail.wed,
    new_dose_detail.thu,
    new_dose_detail.fri,
    new_dose_detail.sat,
    new_dose_detail.sun,
  ];
  let mut day_doses: Vec<f64> = Vec::with_capacity(days as usize);
  for d in 0..days {
    let current = visit + chrono::Duration::days(d);
    let day_index = current.weekday().num_days_from_monday() as usize;
    day_doses.push(weekly[day_index]);
  }
  let pill_counts = tally_pills(&day_doses);

  let lines: Vec<crate::models::visit::PillLineSummary> = pill_counts
    .into_iter()
    .filter(|(_, (dispensed, half))| *dispensed > 0 || *half > 0)
    .map(|(mg, (dispensed, half))| {
      let usage_note = format!(
        "ใช้ {} ยา {} มก. รวม {} เม็ด (ครึ่งเม็ด {} เม็ด)",
        if days >= 28 {
          "รายสัปดาห์"
        } else {
          "รายวัน"
        },
        mg,
        if half > 0 {
          format!("{}+{}", dispensed, half)
        } else {
          dispensed.to_string()
        },
        half
      );
      crate::models::visit::PillLineSummary {
        mg,
        dispensed_count: dispensed,
        usage_note,
      }
    })
    .collect();

  if lines.is_empty() {
    return None;
  }

  let header = format!(
    "รวมยาถึงวันนัด ({} วัน): {} - {}",
    days, visit_date, next_appointment
  );

  Some(crate::models::visit::TotalPillsSummary {
    header,
    pill_lines: lines,
  })
}

fn selected_option_summary(snapshot: &RegimenOptionSnapshot) -> TotalPillsSummary {
  TotalPillsSummary {
    header: snapshot.total_pills_summary.header.clone(),
    pill_lines: snapshot
      .total_pills_summary
      .pill_lines
      .iter()
      .map(|line| crate::models::visit::PillLineSummary {
        mg: line.mg,
        dispensed_count: line.dispensed_count,
        usage_note: line.usage_note.clone(),
      })
      .collect(),
  }
}

fn new_sync_id() -> String {
  Uuid::new_v4().to_string()
}

struct VisitAppointmentLinkContext {
  hn: String,
  next_appointment: Option<String>,
  sync_id: Option<String>,
}

// Pool initialisation

/// Opens (or creates) the SQLite database and runs embedded migrations.
pub async fn init_pool(db_path: PathBuf) -> Result<SqlitePool> {
  let url = format!("sqlite://{}?mode=rwc", db_path.display());
  let pool = SqlitePoolOptions::new()
    .max_connections(5)
    .connect(&url)
    .await
    .with_context(|| format!("failed to open SQLite database at {}", db_path.display()))?;

  sqlx::migrate!("./migrations")
    .run(&pool)
    .await
    .context("failed to run SQLite migrations")?;

  Ok(pool)
}

// wf_patients

/// Inserts a new enrolled patient and returns the new row ID.
pub async fn enroll_patient(
  pool: &SqlitePool,
  input: &EnrollmentInput,
  machine_id: &str,
) -> Result<i64> {
  let now = Utc::now().to_rfc3339();
  let id = sqlx::query(
    "INSERT INTO wf_patients \
         (hn, enrolled_at, enrolled_by, status, indication, \
          target_inr_low, target_inr_high, notes, created_at, updated_at, sync_id, machine_id) \
         VALUES (?, ?, ?, 'active', ?, ?, ?, ?, ?, ?, ?, ?)",
  )
  .bind(&input.hn)
  .bind(&input.enrolled_at)
  .bind(&input.enrolled_by)
  .bind(&input.indication)
  .bind(input.target_inr_low)
  .bind(input.target_inr_high)
  .bind(&input.notes)
  .bind(&now)
  .bind(&now)
  .bind(new_sync_id())
  .bind(machine_id)
  .execute(pool)
  .await
  .context("failed to enroll patient")?
  .last_insert_rowid();

  Ok(id)
}

/// Returns all active warfarin clinic patients.
pub async fn get_active_patients(pool: &SqlitePool) -> Result<Vec<WfPatient>> {
  let rows = sqlx::query(
    "SELECT id, hn, enrolled_at, enrolled_by, status, indication, \
         target_inr_low, target_inr_high, notes, created_at, updated_at \
      FROM wf_patients WHERE status = 'active' AND deleted_at IS NULL ORDER BY enrolled_at DESC",
  )
  .fetch_all(pool)
  .await
  .context("failed to query active patients")?;

  Ok(
    rows
      .iter()
      .map(|r| WfPatient {
        id: r.get("id"),
        hn: r.get("hn"),
        enrolled_at: r.get("enrolled_at"),
        enrolled_by: r.get("enrolled_by"),
        status: r.get("status"),
        indication: r.get("indication"),
        target_inr_low: r.get("target_inr_low"),
        target_inr_high: r.get("target_inr_high"),
        notes: r.get("notes"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
      })
      .collect(),
  )
}

/// Returns all enrolled warfarin clinic patients regardless of status.
pub async fn get_all_patients(pool: &SqlitePool) -> Result<Vec<WfPatient>> {
  let rows = sqlx::query(
    "SELECT id, hn, enrolled_at, enrolled_by, status, indication, \
         target_inr_low, target_inr_high, notes, created_at, updated_at \
      FROM wf_patients WHERE deleted_at IS NULL ORDER BY enrolled_at DESC",
  )
  .fetch_all(pool)
  .await
  .context("failed to query all patients")?;

  Ok(
    rows
      .iter()
      .map(|r| WfPatient {
        id: r.get("id"),
        hn: r.get("hn"),
        enrolled_at: r.get("enrolled_at"),
        enrolled_by: r.get("enrolled_by"),
        status: r.get("status"),
        indication: r.get("indication"),
        target_inr_low: r.get("target_inr_low"),
        target_inr_high: r.get("target_inr_high"),
        notes: r.get("notes"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
      })
      .collect(),
  )
}

/// Fetches a single patient by HN.
pub async fn get_patient_by_hn(pool: &SqlitePool, hn: &str) -> Result<Option<WfPatient>> {
  let row = sqlx::query(
    "SELECT id, hn, enrolled_at, enrolled_by, status, indication, \
         target_inr_low, target_inr_high, notes, created_at, updated_at \
      FROM wf_patients WHERE hn = ? AND deleted_at IS NULL",
  )
  .bind(hn)
  .fetch_optional(pool)
  .await
  .context("failed to query patient")?;

  Ok(row.map(|r| WfPatient {
    id: r.get("id"),
    hn: r.get("hn"),
    enrolled_at: r.get("enrolled_at"),
    enrolled_by: r.get("enrolled_by"),
    status: r.get("status"),
    indication: r.get("indication"),
    target_inr_low: r.get("target_inr_low"),
    target_inr_high: r.get("target_inr_high"),
    notes: r.get("notes"),
    created_at: r.get("created_at"),
    updated_at: r.get("updated_at"),
  }))
}

/// Returns all enrolled HNs (any status).
pub async fn get_all_enrolled_hns(pool: &SqlitePool) -> Result<Vec<String>> {
  let rows = sqlx::query("SELECT hn FROM wf_patients WHERE deleted_at IS NULL")
    .fetch_all(pool)
    .await
    .context("failed to query enrolled HNs")?;
  Ok(rows.iter().map(|r| r.get("hn")).collect())
}

/// Updates a patient's status and records the change metadata.
pub async fn update_patient_status(
  pool: &SqlitePool,
  hn: &str,
  status: &str,
  reason: Option<&str>,
  effective_date: Option<&str>,
  machine_id: &str,
) -> Result<()> {
  let now = Utc::now().to_rfc3339();
  let effective_date = effective_date
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .map(ToOwned::to_owned)
    .unwrap_or_else(|| Utc::now().date_naive().format("%Y-%m-%d").to_string());
  let reason = reason
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .map(ToOwned::to_owned);

  let mut tx = pool
    .begin()
    .await
    .context("failed to begin patient status update transaction")?;

  let result = sqlx::query(
    "UPDATE wf_patients \
        SET status = ?, updated_at = ?, machine_id = ?, sync_id = COALESCE(sync_id, ?) \
      WHERE hn = ? AND deleted_at IS NULL",
  )
  .bind(status)
  .bind(&now)
  .bind(machine_id)
  .bind(new_sync_id())
  .bind(hn)
  .execute(&mut *tx)
  .await
  .context("failed to update patient status")?;

  if result.rows_affected() == 0 {
    bail!("patient not found: {hn}");
  }

  sqlx::query(
    "INSERT INTO wf_patient_status_history \
        (hn, status, reason, effective_date, created_at, updated_at, sync_id, machine_id) \
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
  )
  .bind(hn)
  .bind(status)
  .bind(reason)
  .bind(&effective_date)
  .bind(&now)
  .bind(&now)
  .bind(new_sync_id())
  .bind(machine_id)
  .execute(&mut *tx)
  .await
  .context("failed to record patient status history")?;

  tx.commit()
    .await
    .context("failed to commit patient status update")?;

  Ok(())
}

// wf_visits

/// Inserts a visit record and returns the new row ID.
pub async fn save_visit(pool: &SqlitePool, input: &VisitInput, machine_id: &str) -> Result<i64> {
  if input
    .next_appointment
    .as_deref()
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .is_none()
  {
    bail!("next appointment is required when saving visit");
  }

  let now = Utc::now().to_rfc3339();
  let dose_detail_json = input
    .dose_detail
    .as_ref()
    .map(|d| serde_json::to_string(d).unwrap_or_default());
  let new_dose_detail_json = input
    .new_dose_detail
    .as_ref()
    .map(|d| serde_json::to_string(d).unwrap_or_default());
  let side_effects_json = input
    .side_effects
    .as_ref()
    .filter(|s| !s.is_empty())
    .map(|s| serde_json::to_string(s).unwrap_or_default());
  let selected_dose_option_json = input
    .selected_dose_option
    .as_ref()
    .map(|option| serde_json::to_string(option).unwrap_or_default());
  let dose_changed = i32::from(input.dose_changed);
  let visit_sync_id = new_sync_id();

  let mut tx = pool
    .begin()
    .await
    .context("failed to begin visit save transaction")?;

  let id = sqlx::query(
    "INSERT INTO wf_visits \
         (hn, visit_date, inr_value, inr_source, \
           current_dose_mgday, dose_detail, new_dose_mgday, new_dose_detail, new_dose_description, selected_dose_option, \
           dose_changed, next_appointment, next_inr_due, \
           physician, notes, side_effects, adherence, created_by, created_at, updated_at, sync_id, machine_id) \
          VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
  )
  .bind(&input.hn)
  .bind(&input.visit_date)
  .bind(input.inr_value)
  .bind(&input.inr_source)
  .bind(input.current_dose_mgday)
  .bind(&dose_detail_json)
  .bind(input.new_dose_mgday)
  .bind(&new_dose_detail_json)
  .bind(&input.new_dose_description)
  .bind(&selected_dose_option_json)
  .bind(dose_changed)
  .bind(&input.next_appointment)
  .bind(&input.next_inr_due)
  .bind(&input.physician)
  .bind(&input.notes)
  .bind(&side_effects_json)
  .bind(&input.adherence)
  .bind(&input.created_by)
  .bind(&now)
  .bind(&now)
  .bind(&visit_sync_id)
  .bind(machine_id)
  .execute(&mut *tx)
  .await
  .context("failed to save visit")?
  .last_insert_rowid();

  sync_visit_appointment(&mut tx, input, id, &visit_sync_id, &now, machine_id).await?;

  tx.commit()
    .await
    .context("failed to commit visit save transaction")?;

  Ok(id)
}

/// Updates an existing visit record. Returns error if visit not found.
pub async fn update_visit(
  pool: &SqlitePool,
  visit_id: i64,
  input: &VisitInput,
  machine_id: &str,
) -> Result<()> {
  if input
    .next_appointment
    .as_deref()
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .is_none()
  {
    bail!("next appointment is required when updating visit");
  }

  let now = Utc::now().to_rfc3339();
  let dose_detail_json = input
    .dose_detail
    .as_ref()
    .map(|d| serde_json::to_string(d).unwrap_or_default());
  let new_dose_detail_json = input
    .new_dose_detail
    .as_ref()
    .map(|d| serde_json::to_string(d).unwrap_or_default());
  let side_effects_json = input
    .side_effects
    .as_ref()
    .filter(|s| !s.is_empty())
    .map(|s| serde_json::to_string(s).unwrap_or_default());
  let selected_dose_option_json = input
    .selected_dose_option
    .as_ref()
    .map(|option| serde_json::to_string(option).unwrap_or_default());
  let dose_changed = i32::from(input.dose_changed);

  let mut tx = pool
    .begin()
    .await
    .context("failed to begin visit update transaction")?;
  let existing_link = get_visit_appointment_link_context(&mut tx, visit_id).await?;
  let visit_sync_id = existing_link
    .as_ref()
    .and_then(|context| context.sync_id.clone())
    .unwrap_or_else(new_sync_id);

  let result = sqlx::query(
    "UPDATE wf_visits SET \
        visit_date = ?, inr_value = ?, inr_source = ?, \
        current_dose_mgday = ?, dose_detail = ?, new_dose_mgday = ?, new_dose_detail = ?, new_dose_description = ?, selected_dose_option = ?, \
        dose_changed = ?, next_appointment = ?, next_inr_due = ?, \
      physician = ?, notes = ?, side_effects = ?, adherence = ?, \
      updated_at = ?, machine_id = ?, sync_id = COALESCE(sync_id, ?) \
      WHERE id = ? AND deleted_at IS NULL",
  )
  .bind(&input.visit_date)
  .bind(input.inr_value)
  .bind(&input.inr_source)
  .bind(input.current_dose_mgday)
  .bind(&dose_detail_json)
  .bind(input.new_dose_mgday)
  .bind(&new_dose_detail_json)
  .bind(&input.new_dose_description)
  .bind(&selected_dose_option_json)
  .bind(dose_changed)
  .bind(&input.next_appointment)
  .bind(&input.next_inr_due)
  .bind(&input.physician)
  .bind(&input.notes)
  .bind(&side_effects_json)
  .bind(&input.adherence)
  .bind(&now)
  .bind(machine_id)
  .bind(&visit_sync_id)
  .bind(visit_id)
  .execute(&mut *tx)
  .await
  .context("failed to update visit")?;

  if result.rows_affected() == 0 {
    bail!("visit not found: {visit_id}");
  }

  unlink_or_delete_visit_appointment(
    &mut tx,
    visit_id,
    existing_link.as_ref(),
    &now,
    machine_id,
  )
  .await?;
  sync_visit_appointment(&mut tx, input, visit_id, &visit_sync_id, &now, machine_id).await?;

  tx.commit()
    .await
    .context("failed to commit visit update transaction")?;

  Ok(())
}

/// Returns all visit records for a patient, newest first.
pub async fn get_visit_history(pool: &SqlitePool, hn: &str) -> Result<Vec<WfVisit>> {
  let rows = sqlx::query(
    "SELECT id, hn, visit_date, inr_value, inr_source, \
         current_dose_mgday, dose_detail, new_dose_mgday, new_dose_detail, new_dose_description, selected_dose_option, \
         dose_changed, next_appointment, next_inr_due, \
         physician, notes, side_effects, adherence, created_by, created_at, \
         reviewed_at, reviewed_by \
      FROM wf_visits WHERE hn = ? AND deleted_at IS NULL ORDER BY visit_date DESC",
  )
  .bind(hn)
  .fetch_all(pool)
  .await
  .context("failed to query visit history")?;

  rows
    .iter()
    .map(|r| {
      let dose_detail = r
        .try_get::<Option<String>, _>("dose_detail")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str::<DoseSchedule>(&s).ok());
      let new_dose_detail = r
        .try_get::<Option<String>, _>("new_dose_detail")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str::<DoseSchedule>(&s).ok());
      let side_effects = r
        .try_get::<Option<String>, _>("side_effects")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .filter(|v| !v.is_empty());
      let selected_dose_option = r
        .try_get::<Option<String>, _>("selected_dose_option")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str::<RegimenOptionSnapshot>(&s).ok());
      let dose_changed: i32 = r.try_get("dose_changed").unwrap_or(0);
      let visit_date_str: String = r.get("visit_date");
      let next_appt: Option<String> = r.try_get("next_appointment").ok();
      let total_pills_summary = selected_dose_option
        .as_ref()
        .map(selected_option_summary)
        .or_else(|| {
          if let (Some(na), Some(nd)) = (&next_appt, &new_dose_detail) {
            calculate_pills_summary(&visit_date_str, nd, na)
          } else {
            None
          }
        });

      Ok(WfVisit {
        id: r.get("id"),
        hn: r.get("hn"),
        visit_date: r.get("visit_date"),
        inr_value: r.try_get("inr_value").ok(),
        inr_source: r.try_get("inr_source").ok(),
        current_dose_mgday: r.try_get("current_dose_mgday").ok(),
        dose_detail,
        new_dose_mgday: r.try_get("new_dose_mgday").ok(),
        new_dose_detail,
        new_dose_description: r.try_get("new_dose_description").ok(),
        dose_changed: dose_changed != 0,
        next_appointment: r.try_get("next_appointment").ok(),
        next_inr_due: r.try_get("next_inr_due").ok(),
        physician: r.try_get("physician").ok(),
        notes: r.try_get("notes").ok(),
        side_effects,
        adherence: r.try_get("adherence").ok(),
        created_by: r.try_get("created_by").ok(),
        created_at: r.get("created_at"),
        total_pills_summary,
        selected_dose_option,
        reviewed_at: r.try_get("reviewed_at").ok(),
        reviewed_by: r.try_get("reviewed_by").ok(),
      })
    })
    .collect()
}

pub async fn get_visit_by_id(pool: &SqlitePool, visit_id: i64) -> Result<Option<WfVisit>> {
  let row = sqlx::query(
    "SELECT id, hn, visit_date, inr_value, inr_source, \
         current_dose_mgday, dose_detail, new_dose_mgday, new_dose_detail, new_dose_description, selected_dose_option, \
         dose_changed, next_appointment, next_inr_due, \
         physician, notes, side_effects, adherence, created_by, created_at, \
         reviewed_at, reviewed_by \
      FROM wf_visits WHERE id = ? AND deleted_at IS NULL",
  )
  .bind(visit_id)
  .fetch_optional(pool)
  .await
  .context("failed to query visit by id")?;

  Ok(row.as_ref().map(|r| {
    let dose_detail = r
      .try_get::<Option<String>, _>("dose_detail")
      .ok()
      .flatten()
      .and_then(|s| serde_json::from_str(&s).ok());
    let new_dose_detail = r
      .try_get::<Option<String>, _>("new_dose_detail")
      .ok()
      .flatten()
      .and_then(|s| serde_json::from_str::<DoseSchedule>(&s).ok());
    let side_effects = r
      .try_get::<Option<String>, _>("side_effects")
      .ok()
      .flatten()
      .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
      .filter(|v| !v.is_empty());
    let selected_dose_option = r
      .try_get::<Option<String>, _>("selected_dose_option")
      .ok()
      .flatten()
      .and_then(|s| serde_json::from_str::<RegimenOptionSnapshot>(&s).ok());
    let dose_changed: i32 = r.try_get("dose_changed").unwrap_or(0);
    let visit_date_str: String = r.get("visit_date");
    let next_appt_str: Option<String> = r.try_get("next_appointment").ok();
    let new_dose_desc: Option<String> = r.try_get("new_dose_description").ok();

    let total_pills_summary = selected_dose_option
      .as_ref()
      .map(selected_option_summary)
      .or_else(|| {
        if let (Some(vd), Some(na), Some(nd)) =
          (Some(&visit_date_str), &next_appt_str, &new_dose_detail)
        {
          calculate_pills_summary(vd, nd, na)
        } else {
          None
        }
      });

    WfVisit {
      id: r.get("id"),
      hn: r.get("hn"),
      visit_date: visit_date_str,
      inr_value: r.try_get("inr_value").ok(),
      inr_source: r.try_get("inr_source").ok(),
      current_dose_mgday: r.try_get("current_dose_mgday").ok(),
      dose_detail,
      new_dose_mgday: r.try_get("new_dose_mgday").ok(),
      new_dose_detail,
      new_dose_description: new_dose_desc,
      dose_changed: dose_changed != 0,
      next_appointment: next_appt_str,
      next_inr_due: r.try_get("next_inr_due").ok(),
      physician: r.try_get("physician").ok(),
      notes: r.try_get("notes").ok(),
      side_effects,
      adherence: r.try_get("adherence").ok(),
      created_by: r.try_get("created_by").ok(),
      created_at: r.get("created_at"),
      total_pills_summary,
      selected_dose_option,
      reviewed_at: r.try_get("reviewed_at").ok(),
      reviewed_by: r.try_get("reviewed_by").ok(),
    }
  }))
}

pub async fn get_latest_visit_dose_by_hns(
  pool: &SqlitePool,
  hns: &[String],
) -> Result<std::collections::HashMap<String, Option<f64>>> {
  if hns.is_empty() {
    return Ok(std::collections::HashMap::new());
  }

  let mut builder = QueryBuilder::<Sqlite>::new(
    "SELECT hn, dose FROM (\
       SELECT hn, COALESCE(new_dose_mgday, current_dose_mgday) AS dose, \
              ROW_NUMBER() OVER (PARTITION BY hn ORDER BY visit_date DESC, id DESC) AS row_num \
         FROM wf_visits \
        WHERE deleted_at IS NULL AND hn IN (",
  );
  {
    let mut separated = builder.separated(", ");
    for hn in hns {
      separated.push_bind(hn);
    }
  }
  builder.push(") ) ranked WHERE row_num = 1");

  let rows = builder
    .build()
    .fetch_all(pool)
    .await
    .context("failed to query latest visit dose by HN")?;

  Ok(
    rows
      .into_iter()
      .map(|row| {
        let hn: String = row.get("hn");
        let dose = row.try_get::<Option<f64>, _>("dose").ok().flatten();
        (hn, dose)
      })
      .collect(),
  )
}

/// Returns INR values recorded via the clinic visit form (fallback).
pub async fn get_inr_from_visits(pool: &SqlitePool, hn: &str) -> Result<Vec<InrRecord>> {
  let rows = sqlx::query(
    "SELECT visit_date, inr_value, inr_source FROM wf_visits \
      WHERE hn = ? AND deleted_at IS NULL AND inr_value IS NOT NULL ORDER BY visit_date ASC",
  )
  .bind(hn)
  .fetch_all(pool)
  .await
  .context("failed to query INR from visits")?;

  Ok(
    rows
      .iter()
      .filter_map(|r| {
        let value: Option<f64> = r.try_get("inr_value").ok();
        value.map(|v| InrRecord {
          date: r.get("visit_date"),
          value: v,
          source: r
            .try_get::<Option<String>, _>("inr_source")
            .ok()
            .flatten()
            .unwrap_or_else(|| "manual".to_string()),
          lab_order_number: None,
          vn: None,
        })
      })
      .collect(),
  )
}

/// Deletes a visit record by ID.
pub async fn delete_visit(pool: &SqlitePool, visit_id: i64, machine_id: &str) -> Result<()> {
  let now = Utc::now().to_rfc3339();
  let mut tx = pool
    .begin()
    .await
    .context("failed to begin visit delete transaction")?;
  let visit_link = get_visit_appointment_link_context(&mut tx, visit_id).await?;

  unlink_or_delete_visit_appointment(
    &mut tx,
    visit_id,
    visit_link.as_ref(),
    &now,
    machine_id,
  )
  .await?;

  let result = sqlx::query(
    "UPDATE wf_visits \
        SET deleted_at = ?, updated_at = ?, machine_id = ?, sync_id = COALESCE(sync_id, ?) \
      WHERE id = ? AND deleted_at IS NULL",
  )
  .bind(&now)
  .bind(&now)
  .bind(machine_id)
  .bind(new_sync_id())
  .bind(visit_id)
  .execute(&mut *tx)
  .await
  .context("failed to delete visit")?;

  if result.rows_affected() == 0 {
    bail!("visit not found: {visit_id}");
  }

  tx.commit()
    .await
    .context("failed to commit visit delete transaction")?;

  Ok(())
}

async fn get_visit_appointment_link_context(
  tx: &mut Transaction<'_, Sqlite>,
  visit_id: i64,
) -> Result<Option<VisitAppointmentLinkContext>> {
  let row = sqlx::query("SELECT hn, next_appointment, sync_id FROM wf_visits WHERE id = ? LIMIT 1")
    .bind(visit_id)
    .fetch_optional(&mut **tx)
    .await
    .context("failed to query visit appointment link context")?;

  Ok(row.map(|row| VisitAppointmentLinkContext {
    hn: row.get("hn"),
    next_appointment: row.try_get("next_appointment").ok().flatten(),
    sync_id: row.try_get("sync_id").ok().flatten(),
  }))
}

async fn sync_visit_appointment(
  tx: &mut Transaction<'_, Sqlite>,
  input: &VisitInput,
  visit_id: i64,
  visit_sync_id: &str,
  now: &str,
  machine_id: &str,
) -> Result<()> {
  let Some(next_appointment) = input
    .next_appointment
    .as_deref()
    .map(str::trim)
    .filter(|value| !value.is_empty())
  else {
    return Ok(());
  };

  let existing_manual_appointment_id = sqlx::query_scalar::<_, i64>(
    "SELECT id FROM wf_appointments \
      WHERE deleted_at IS NULL AND (\
            (hn = ? AND appt_date = ? AND status = 'scheduled' \
              AND source_visit_id IS NULL AND source_visit_sync_id IS NULL)\
         OR source_visit_sync_id = ?) \
      ORDER BY CASE \
          WHEN source_visit_sync_id = ? THEN 0 \
          ELSE 1 \
        END, id DESC LIMIT 1",
  )
  .bind(&input.hn)
  .bind(next_appointment)
  .bind(visit_sync_id)
  .bind(visit_sync_id)
  .fetch_optional(&mut **tx)
  .await
  .context("failed to find reusable appointment for visit")?;

  if let Some(appointment_id) = existing_manual_appointment_id {
    sqlx::query(
      "UPDATE wf_appointments \
           SET source_visit_id = ?, \
           source_visit_sync_id = ?, \
           appt_type = COALESCE(appt_type, 'clinic_visit'), \
           updated_at = ?, \
           machine_id = ?, \
           sync_id = COALESCE(sync_id, ?) \
         WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(visit_id)
    .bind(visit_sync_id)
    .bind(now)
    .bind(machine_id)
    .bind(new_sync_id())
    .bind(appointment_id)
    .execute(&mut **tx)
    .await
    .context("failed to link existing appointment to visit")?;
  } else {
    sqlx::query(
      "INSERT INTO wf_appointments \
        (hn, appt_date, appt_type, status, notes, created_at, updated_at, source_visit_id, source_visit_sync_id, generated_from_visit, sync_id, machine_id) \
        VALUES (?, ?, 'clinic_visit', 'scheduled', NULL, ?, ?, ?, ?, 1, ?, ?)",
    )
    .bind(&input.hn)
    .bind(next_appointment)
    .bind(now)
    .bind(now)
    .bind(visit_id)
    .bind(visit_sync_id)
    .bind(new_sync_id())
    .bind(machine_id)
    .execute(&mut **tx)
    .await
    .context("failed to create linked appointment for visit")?;
  }

  Ok(())
}

async fn unlink_or_delete_visit_appointment(
  tx: &mut Transaction<'_, Sqlite>,
  visit_id: i64,
  visit_link: Option<&VisitAppointmentLinkContext>,
  now: &str,
  machine_id: &str,
) -> Result<()> {
  let visit_sync_id = visit_link.and_then(|link| link.sync_id.as_deref());
  let visit_hn = visit_link.map(|link| link.hn.as_str());
  let visit_next_appointment = visit_link.and_then(|link| link.next_appointment.as_deref());

  let linked_appointment = sqlx::query(
    "SELECT id, generated_from_visit FROM wf_appointments \
      WHERE deleted_at IS NULL AND (\
            source_visit_sync_id = ? \
         OR (source_visit_sync_id IS NULL AND source_visit_id = ? AND hn = ? AND appt_date = ?)\
      ) \
      ORDER BY CASE WHEN source_visit_sync_id = ? THEN 0 ELSE 1 END LIMIT 1",
  )
  .bind(visit_sync_id)
  .bind(visit_id)
  .bind(visit_hn)
  .bind(visit_next_appointment)
  .bind(visit_sync_id)
  .fetch_optional(&mut **tx)
  .await
  .context("failed to query linked appointment for visit")?;

  let Some(appointment) = linked_appointment else {
    return Ok(());
  };

  let appointment_id: i64 = appointment.get("id");
  let generated_from_visit: i32 = appointment.try_get("generated_from_visit").unwrap_or(0);

  if generated_from_visit != 0 {
    sqlx::query(
      "UPDATE wf_appointments \
          SET deleted_at = ?, source_visit_id = NULL, source_visit_sync_id = NULL, updated_at = ?, machine_id = ?, sync_id = COALESCE(sync_id, ?) \
        WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(now)
    .bind(now)
    .bind(machine_id)
    .bind(new_sync_id())
    .bind(appointment_id)
    .execute(&mut **tx)
    .await
    .context("failed to delete auto-generated appointment for visit")?;
  } else {
    sqlx::query(
      "UPDATE wf_appointments \
          SET source_visit_id = NULL, source_visit_sync_id = NULL, updated_at = ?, machine_id = ?, sync_id = COALESCE(sync_id, ?) \
        WHERE id = ? AND deleted_at IS NULL",
    )
      .bind(now)
      .bind(machine_id)
      .bind(new_sync_id())
      .bind(appointment_id)
      .execute(&mut **tx)
      .await
      .context("failed to unlink manual appointment from visit")?;
  }

  Ok(())
}

// wf_appointments

/// Inserts a new appointment and returns the new row ID.
pub async fn schedule_appointment(
  pool: &SqlitePool,
  input: &AppointmentInput,
  machine_id: &str,
) -> Result<i64> {
  let now = Utc::now().to_rfc3339();
  let id = sqlx::query(
    "INSERT INTO wf_appointments \
         (hn, appt_date, appt_type, status, notes, created_at, updated_at, sync_id, machine_id) \
         VALUES (?, ?, ?, 'scheduled', ?, ?, ?, ?, ?)",
  )
  .bind(&input.hn)
  .bind(&input.appt_date)
  .bind(&input.appt_type)
  .bind(&input.notes)
  .bind(&now)
  .bind(&now)
  .bind(new_sync_id())
  .bind(machine_id)
  .execute(pool)
  .await
  .context("failed to schedule appointment")?
  .last_insert_rowid();

  Ok(id)
}

/// Returns all appointments for a patient, sorted by date.
pub async fn get_appointments(pool: &SqlitePool, hn: &str) -> Result<Vec<WfAppointment>> {
  let rows = sqlx::query(
    "SELECT id, hn, appt_date, appt_type, status, notes, created_at \
      FROM wf_appointments WHERE hn = ? AND deleted_at IS NULL ORDER BY appt_date ASC",
  )
  .bind(hn)
  .fetch_all(pool)
  .await
  .context("failed to query appointments")?;

  Ok(
    rows
      .iter()
      .map(|r| WfAppointment {
        id: r.get("id"),
        hn: r.get("hn"),
        appt_date: r.get("appt_date"),
        appt_type: r.try_get("appt_type").ok(),
        status: r.get("status"),
        notes: r.try_get("notes").ok(),
        created_at: r.get("created_at"),
      })
      .collect(),
  )
}

/// Returns all pending (scheduled) appointments across all patients.
pub async fn get_pending_appointments(pool: &SqlitePool) -> Result<Vec<WfAppointment>> {
  let rows = sqlx::query(
    "SELECT id, hn, appt_date, appt_type, status, notes, created_at \
      FROM wf_appointments WHERE status = 'scheduled' AND deleted_at IS NULL ORDER BY appt_date ASC",
  )
  .fetch_all(pool)
  .await
  .context("failed to query pending appointments")?;

  Ok(
    rows
      .iter()
      .map(|r| WfAppointment {
        id: r.get("id"),
        hn: r.get("hn"),
        appt_date: r.get("appt_date"),
        appt_type: r.try_get("appt_type").ok(),
        status: r.get("status"),
        notes: r.try_get("notes").ok(),
        created_at: r.get("created_at"),
      })
      .collect(),
  )
}

pub async fn get_appointment_day_load(
  pool: &SqlitePool,
  appt_date: &str,
) -> Result<AppointmentDayLoad> {
  let scheduled_count = sqlx::query_scalar::<_, i64>(
    "SELECT COUNT(DISTINCT hn) FROM wf_appointments \
      WHERE appt_date = ? AND status = 'scheduled' AND deleted_at IS NULL",
  )
  .bind(appt_date)
  .fetch_one(pool)
  .await
  .context("failed to query appointment day load")?;

  Ok(AppointmentDayLoad {
    appt_date: appt_date.to_string(),
    scheduled_count,
  })
}

// wf_outcomes

pub async fn record_adverse_event(
  pool: &SqlitePool,
  input: &OutcomeInput,
  machine_id: &str,
) -> Result<i64> {
  let now = Utc::now().to_rfc3339();
  let id = sqlx::query(
    "INSERT INTO wf_outcomes \
         (hn, event_date, event_type, description, inr_at_event, action_taken, created_by, created_at, updated_at, sync_id, machine_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
  )
  .bind(&input.hn)
  .bind(&input.event_date)
  .bind(&input.event_type)
  .bind(&input.description)
  .bind(input.inr_at_event)
  .bind(&input.action_taken)
  .bind(&input.created_by)
  .bind(&now)
  .bind(&now)
  .bind(new_sync_id())
  .bind(machine_id)
  .execute(pool)
  .await
  .context("failed to record adverse event")?
  .last_insert_rowid();

  Ok(id)
}

pub async fn get_outcomes(pool: &SqlitePool, hn: &str) -> Result<Vec<WfOutcome>> {
  let rows = sqlx::query(
    "SELECT id, hn, event_date, event_type, description, inr_at_event, action_taken, created_by, created_at \
      FROM wf_outcomes WHERE hn = ? AND deleted_at IS NULL ORDER BY event_date DESC, id DESC",
  )
  .bind(hn)
  .fetch_all(pool)
  .await
  .context("failed to query outcomes")?;

  Ok(
    rows
      .iter()
      .map(|r| WfOutcome {
        id: r.get("id"),
        hn: r.get("hn"),
        event_date: r.get("event_date"),
        event_type: r.get("event_type"),
        description: r.try_get("description").ok(),
        inr_at_event: r.try_get("inr_at_event").ok(),
        action_taken: r.try_get("action_taken").ok(),
        created_by: r.try_get("created_by").ok(),
        created_at: r.get("created_at"),
      })
      .collect(),
  )
}

pub async fn get_outcome_counts_by_hns(
  pool: &SqlitePool,
  hns: &[String],
) -> Result<std::collections::HashMap<String, i64>> {
  if hns.is_empty() {
    return Ok(std::collections::HashMap::new());
  }

  let mut builder = QueryBuilder::<Sqlite>::new(
    "SELECT hn, COUNT(*) AS outcome_count FROM wf_outcomes WHERE deleted_at IS NULL AND hn IN (",
  );
  {
    let mut separated = builder.separated(", ");
    for hn in hns {
      separated.push_bind(hn);
    }
  }
  builder.push(") GROUP BY hn");

  let rows = builder
    .build()
    .fetch_all(pool)
    .await
    .context("failed to query outcome counts by HN")?;

  Ok(
    rows
      .into_iter()
      .map(|row| (row.get("hn"), row.get("outcome_count")))
      .collect(),
  )
}

// wf_settings

/// Fetches all settings as key-value pairs.
pub async fn get_all_settings(pool: &SqlitePool) -> Result<Vec<(String, String)>> {
  let rows = sqlx::query("SELECT key, value FROM wf_settings ORDER BY key")
    .fetch_all(pool)
    .await
    .context("failed to query settings")?;
  Ok(
    rows
      .iter()
      .map(|r| (r.get("key"), r.get("value")))
      .collect(),
  )
}

/// Upserts a setting value.
pub async fn set_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<()> {
  sqlx::query(
    "INSERT INTO wf_settings (key, value) VALUES (?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
  )
  .bind(key)
  .bind(value)
  .execute(pool)
  .await
  .context("failed to upsert setting")?;
  Ok(())
}

/// Fetches a single setting by key.
pub async fn get_setting(pool: &SqlitePool, key: &str) -> Result<Option<String>> {
  let row = sqlx::query("SELECT value FROM wf_settings WHERE key = ?")
    .bind(key)
    .fetch_optional(pool)
    .await
    .context("failed to query setting")?;
  Ok(row.map(|r| r.get("value")))
}

// wf_drug_interactions

/// Fetches all drug interactions configured in the system.
pub async fn get_all_drug_interactions(pool: &SqlitePool) -> Result<Vec<DrugInteraction>> {
  let rows = sqlx::query(
    "SELECT id, icode, drug_name, strength, interaction_type, created_at, updated_at \
         FROM wf_drug_interactions ORDER BY drug_name, icode",
  )
  .fetch_all(pool)
  .await
  .context("failed to query drug interactions")?;

  Ok(
    rows
      .iter()
      .map(|r| DrugInteraction {
        id: r.get("id"),
        icode: r.get("icode"),
        drug_name: r.get("drug_name"),
        strength: r.try_get("strength").ok(),
        interaction_type: r.get("interaction_type"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
      })
      .collect(),
  )
}

/// Returns all configured drug interaction icodes.
pub async fn get_drug_interaction_icodes(pool: &SqlitePool) -> Result<Vec<String>> {
  let rows = sqlx::query("SELECT icode FROM wf_drug_interactions")
    .fetch_all(pool)
    .await
    .context("failed to query drug interaction icodes")?;
  Ok(rows.iter().map(|r| r.get("icode")).collect())
}

/// Adds a new drug interaction.
pub async fn add_drug_interaction(pool: &SqlitePool, input: &DrugInteractionInput) -> Result<i64> {
  let now = Utc::now().to_rfc3339();
  let id = sqlx::query(
    "INSERT INTO wf_drug_interactions \
         (icode, drug_name, strength, interaction_type, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?)",
  )
  .bind(&input.icode)
  .bind(&input.drug_name)
  .bind(&input.strength)
  .bind(&input.interaction_type)
  .bind(&now)
  .bind(&now)
  .execute(pool)
  .await
  .context("failed to add drug interaction")?
  .last_insert_rowid();

  Ok(id)
}

/// Deletes a drug interaction by ID.
pub async fn delete_drug_interaction(pool: &SqlitePool, id: i64) -> Result<()> {
  let result = sqlx::query("DELETE FROM wf_drug_interactions WHERE id = ?")
    .bind(id)
    .execute(pool)
    .await
    .context("failed to delete drug interaction")?;

  if result.rows_affected() == 0 {
    bail!("drug interaction not found: {id}");
  }

  Ok(())
}

/// Returns visits pending review (no reviewed_at), newest first.
pub async fn get_pending_review_visits(pool: &SqlitePool) -> Result<Vec<WfVisit>> {
  let rows = sqlx::query(
    "SELECT id, hn, visit_date, inr_value, inr_source, \
         current_dose_mgday, dose_detail, new_dose_mgday, new_dose_detail, new_dose_description, selected_dose_option, \
         dose_changed, next_appointment, next_inr_due, \
         physician, notes, side_effects, adherence, created_by, created_at, \
         reviewed_at, reviewed_by \
      FROM wf_visits WHERE reviewed_at IS NULL AND deleted_at IS NULL ORDER BY visit_date DESC",
  )
  .fetch_all(pool)
  .await
  .context("failed to query pending review visits")?;

  rows
    .iter()
    .map(|r| {
      let dose_detail = r
        .try_get::<Option<String>, _>("dose_detail")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str::<DoseSchedule>(&s).ok());
      let new_dose_detail = r
        .try_get::<Option<String>, _>("new_dose_detail")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str::<DoseSchedule>(&s).ok());
      let side_effects = r
        .try_get::<Option<String>, _>("side_effects")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .filter(|v| !v.is_empty());
      let selected_dose_option = r
        .try_get::<Option<String>, _>("selected_dose_option")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str::<RegimenOptionSnapshot>(&s).ok());
      let dose_changed: i32 = r.try_get("dose_changed").unwrap_or(0);
      let visit_date_str: String = r.get("visit_date");
      let next_appt: Option<String> = r.try_get("next_appointment").ok();
      let total_pills_summary = selected_dose_option
        .as_ref()
        .map(selected_option_summary)
        .or_else(|| {
          if let (Some(na), Some(nd)) = (&next_appt, &new_dose_detail) {
            calculate_pills_summary(&visit_date_str, nd, na)
          } else {
            None
          }
        });

      Ok(WfVisit {
        id: r.get("id"),
        hn: r.get("hn"),
        visit_date: r.get("visit_date"),
        inr_value: r.try_get("inr_value").ok(),
        inr_source: r.try_get("inr_source").ok(),
        current_dose_mgday: r.try_get("current_dose_mgday").ok(),
        dose_detail,
        new_dose_mgday: r.try_get("new_dose_mgday").ok(),
        new_dose_detail,
        new_dose_description: r.try_get("new_dose_description").ok(),
        dose_changed: dose_changed != 0,
        next_appointment: r.try_get("next_appointment").ok(),
        next_inr_due: r.try_get("next_inr_due").ok(),
        physician: r.try_get("physician").ok(),
        notes: r.try_get("notes").ok(),
        side_effects,
        adherence: r.try_get("adherence").ok(),
        created_by: r.try_get("created_by").ok(),
        created_at: r.get("created_at"),
        total_pills_summary,
        selected_dose_option,
        reviewed_at: r.try_get("reviewed_at").ok(),
        reviewed_by: r.try_get("reviewed_by").ok(),
      })
    })
    .collect()
}

/// Returns count of visits pending review.
pub async fn get_pending_review_count(pool: &SqlitePool) -> Result<i64> {
  let row = sqlx::query(
    "SELECT COUNT(*) as cnt FROM wf_visits WHERE reviewed_at IS NULL AND deleted_at IS NULL",
  )
  .fetch_one(pool)
  .await
  .context("failed to count pending reviews")?;
  Ok(row.get("cnt"))
}

/// Approves a visit (sets reviewed_at and reviewed_by).
pub async fn approve_visit(
  pool: &SqlitePool,
  visit_id: i64,
  reviewer: &str,
  machine_id: &str,
) -> Result<()> {
  let now = Utc::now().to_rfc3339();
  let result = sqlx::query(
    "UPDATE wf_visits \
        SET reviewed_at = ?, reviewed_by = ?, updated_at = ?, machine_id = ?, sync_id = COALESCE(sync_id, ?) \
      WHERE id = ? AND deleted_at IS NULL",
  )
    .bind(&now)
    .bind(reviewer)
    .bind(&now)
    .bind(machine_id)
    .bind(new_sync_id())
    .bind(visit_id)
    .execute(pool)
    .await
    .context("failed to approve visit")?;

  if result.rows_affected() == 0 {
    bail!("visit not found: {visit_id}");
  }

  Ok(())
}

// AppState

/// Application state managed by Tauri, wrapping the SQLite connection pool.
///
/// Registered with `tauri::Builder::manage()` and injected into every command
/// handler via `tauri::State<'_, AppState>`.
pub struct AppState {
  /// SQLite connection pool.
  pub pool: SqlitePool,
  /// Stable identifier for the current machine, used for sync metadata.
  pub machine_id: String,
}

impl AppState {
  /// Constructs `AppState` from an already-initialised pool.
  pub fn new(pool: SqlitePool, machine_id: String) -> Self {
    Self { pool, machine_id }
  }
}

#[cfg(test)]
mod pill_counter_tests {
  use super::tally_pills;

  #[test]
  fn one_day_5mg_is_one_5mg_pill() {
    let counts = tally_pills(&[5.0]);
    assert_eq!(counts.get(&5).copied(), Some((1, 0)));
  }

  #[test]
  fn one_day_2_5mg_is_half_5mg_pill() {
    // 2.5 mg = ½ × 5 mg pill (single half-pill is the fewest pieces).
    let counts = tally_pills(&[2.5]);
    assert_eq!(counts.get(&5).copied(), Some((0, 1)));
  }

  #[test]
  fn one_day_3_5mg_is_two_halves() {
    // 3.5 mg is exactly ½ × 5 mg + ½ × 2 mg (2.5 + 1.0 = 3.5). 2 pieces.
    let counts = tally_pills(&[3.5]);
    assert_eq!(counts.get(&5).copied(), Some((0, 1)));
    assert_eq!(counts.get(&2).copied(), Some((0, 1)));
  }

  #[test]
  fn one_day_4mg_picks_a_valid_split() {
    // 4 mg = 1 × 3 + ½ × 2 OR 2 × 2 OR ½×5 + ½×3 (all 2 pieces, all 4 mg).
    let counts = tally_pills(&[4.0]);
    let total_half_units: i32 = counts
      .iter()
      .map(|(s, (w, h))| (*s as i32) * 2 * (*w as i32) + (*s as i32) * (*h as i32))
      .sum();
    assert_eq!(total_half_units, 8, "4 mg dose should dispense 4 mg total");
    let total_pieces: u32 = counts.values().map(|(w, h)| w + h).sum();
    assert_eq!(total_pieces, 2, "should use 2 pieces (minimum)");
  }

  #[test]
  fn one_day_4_5mg_picks_a_valid_split() {
    // 4.5 mg = 1 × 3 + ½ × 3 OR ½ × 5 + 1 × 2 (both 2 pieces, both 4.5 mg).
    let counts = tally_pills(&[4.5]);
    let total_half_units: i32 = counts
      .iter()
      .map(|(s, (w, h))| (*s as i32) * 2 * (*w as i32) + (*s as i32) * (*h as i32))
      .sum();
    assert_eq!(total_half_units, 9, "4.5 mg dose should dispense 4.5 mg total");
    let total_pieces: u32 = counts.values().map(|(w, h)| w + h).sum();
    assert_eq!(total_pieces, 2, "should use 2 pieces (minimum)");
  }

  #[test]
  fn multi_week_schedule_sums_correctly() {
    // Two weeks of "5 mg Mon-Fri" = 10 whole 5 mg pills.
    let doses = vec![5.0, 5.0, 5.0, 5.0, 5.0, 0.0, 0.0];
    let doses: Vec<f64> = doses.iter().cycle().take(14).copied().collect();
    let counts = tally_pills(&doses);
    assert_eq!(counts.get(&5).copied(), Some((10, 0)));
  }

  #[test]
  fn fractional_dose_is_quantised() {
    // 2.6 mg rounds to 2.5 mg = half of 5 mg.
    let counts = tally_pills(&[2.6]);
    assert_eq!(counts.get(&5).copied(), Some((0, 1)));
  }

  #[test]
  fn zero_dose_contributes_nothing() {
    let counts = tally_pills(&[0.0, 0.0, 0.0]);
    assert!(counts.is_empty());
  }

  #[test]
  fn mixed_week_schedule() {
    // Mon 5mg, Tue 2.5mg, Wed 5mg, Thu 0, Fri 3.5mg, Sat 0, Sun 0
    //   5mg:   1×5     0        1×5     0     0       0     0    → 2 whole, 0 half
    //   2.5mg: 0       ½×5      0       0     0       0     0    → 0 whole, 1 half
    //   3.5mg: 0       0        0       0     ½×5+½×2 0     0  → 0 whole, 1 half (5) + 1 half (2)
    // Totals: 2 whole 5mg, 2 half 5mg, 0 whole 2mg, 1 half 2mg.
    let counts = tally_pills(&[5.0, 2.5, 5.0, 0.0, 3.5, 0.0, 0.0]);
    assert_eq!(counts.get(&5).copied(), Some((2, 2)));
    assert_eq!(counts.get(&2).copied(), Some((0, 1)));
  }
}
