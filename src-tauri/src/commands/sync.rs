use anyhow::Context;
use reqwest::{Client, Url};
use serde_json::json;
use sqlx::{QueryBuilder, Sqlite};
use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;
use uuid::Uuid;

use crate::db::sqlite::AppState;
use crate::{
  encrypt::{decrypt_value, encrypt_value},
  models::sync::{
    SyncResult, SyncStatus, SyncSummary, WfAppointmentSync, WfDoseHistorySync, WfOutcomeSync,
    WfPatientStatusHistorySync, WfPatientSync, WfVisitSync,
  },
};

const STORE_FILE: &str = "config.json";
const SUPABASE_URL_KEY: &str = "supabase_url";
const SUPABASE_ANON_KEY_KEY: &str = "supabase_anon_key_enc";
const MACHINE_ID_KEY: &str = "machine_id";
const LAST_PULL_AT_KEY: &str = "last_pull_at";
const LAST_SYNC_AT_KEY: &str = "last_sync_at";

pub(crate) fn get_or_create_machine_id(app: &AppHandle) -> Result<String, String> {
  let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
  if let Some(machine_id) = store
    .get(MACHINE_ID_KEY)
    .and_then(|value| value.as_str().map(str::to_owned))
  {
    return Ok(machine_id);
  }

  let machine_id = Uuid::new_v4().to_string();
  store.set(MACHINE_ID_KEY, json!(machine_id));
  store.save().map_err(|e| e.to_string())?;
  Ok(machine_id)
}

fn get_supabase_config(app: &AppHandle) -> Result<(String, String), String> {
  let machine_id = get_or_create_machine_id(app)?;
  let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;

  let url = store
    .get(SUPABASE_URL_KEY)
    .and_then(|value| value.as_str().map(str::to_owned))
    .as_deref()
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .ok_or_else(|| "Supabase URL is not configured".to_string())?
    .trim_end_matches('/')
    .to_string();

  let encrypted_key = store
    .get(SUPABASE_ANON_KEY_KEY)
    .and_then(|value| value.as_str().map(str::to_owned))
    .ok_or_else(|| "Supabase anon key is not configured".to_string())?;

  let anon_key = decrypt_value(&encrypted_key, &machine_id)?;
  Ok((url, anon_key))
}

fn supabase_client() -> Client {
  Client::new()
}

fn build_rest_url(base_url: &str, table: &str, query: &[(&str, String)]) -> Result<Url, String> {
  let mut url = Url::parse(&format!(
    "{}/rest/v1/{}",
    base_url.trim_end_matches('/'),
    table
  ))
  .map_err(|e| e.to_string())?;
  if !query.is_empty() {
    url
      .query_pairs_mut()
      .extend_pairs(query.iter().map(|(key, value)| (*key, value.as_str())));
  }
  Ok(url)
}

fn with_auth(
  builder: reqwest::RequestBuilder,
  anon_key: &str,
  machine_id: &str,
) -> reqwest::RequestBuilder {
  builder
    .header("apikey", anon_key)
    .header("Authorization", format!("Bearer {anon_key}"))
    .header("x-machine-id", machine_id)
}

async fn ensure_sync_ids(
  pool: &sqlx::SqlitePool,
  table: &str,
  machine_id: &str,
) -> Result<(), String> {
  let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
  let select_sql = format!("SELECT id FROM {table} WHERE sync_id IS NULL");
  let ids = sqlx::query_scalar::<_, i64>(&select_sql)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

  for row_id in ids {
    let update_sql =
      format!("UPDATE {table} SET sync_id = ?, machine_id = COALESCE(machine_id, ?) WHERE id = ?");
    sqlx::query(&update_sql)
      .bind(Uuid::new_v4().to_string())
      .bind(machine_id)
      .bind(row_id)
      .execute(&mut *tx)
      .await
      .map_err(|e| e.to_string())?;
  }

  tx.commit().await.map_err(|e| e.to_string())?;
  Ok(())
}

fn sync_ids_from_rows<T>(rows: &[T], get_sync_id: impl Fn(&T) -> Option<&String>) -> Vec<String> {
  rows.iter().filter_map(get_sync_id).cloned().collect()
}

async fn mark_rows_synced(
  pool: &sqlx::SqlitePool,
  table: &str,
  sync_ids: &[String],
  synced_at: &str,
) -> Result<(), String> {
  if sync_ids.is_empty() {
    return Ok(());
  }

  let mut builder = QueryBuilder::<Sqlite>::new(format!("UPDATE {table} SET synced_at = "));
  builder.push_bind(synced_at).push(" WHERE sync_id IN (");
  {
    let mut separated = builder.separated(", ");
    for sync_id in sync_ids {
      separated.push_bind(sync_id);
    }
  }
  builder.push(")");

  builder
    .build()
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
  Ok(())
}

async fn resolve_local_visit_id(
  pool: &sqlx::SqlitePool,
  source_visit_sync_id: Option<&str>,
) -> Result<Option<i64>, String> {
  let Some(source_visit_sync_id) = source_visit_sync_id
    .map(str::trim)
    .filter(|value| !value.is_empty())
  else {
    return Ok(None);
  };

  sqlx::query_scalar::<_, i64>(
    "SELECT id FROM wf_visits WHERE sync_id = ? AND deleted_at IS NULL LIMIT 1",
  )
  .bind(source_visit_sync_id)
  .fetch_optional(pool)
  .await
  .map_err(|e| e.to_string())
}

async fn push_rows<T>(
  client: &Client,
  url: &str,
  anon_key: &str,
  machine_id: &str,
  table: &str,
  conflict_target: &str,
  rows: &[T],
) -> Result<(), String>
where
  T: serde::Serialize + Sync,
{
  if rows.is_empty() {
    return Ok(());
  }

  let endpoint = build_rest_url(url, table, &[("on_conflict", conflict_target.to_string())])?;
  let response = with_auth(client.post(endpoint), anon_key, machine_id)
    .header("Prefer", "resolution=merge-duplicates,return=minimal")
    .json(rows)
    .send()
    .await
    .map_err(|e| format!("[{}] Network error: {}", table, e))?;

  if response.status().is_success() {
    return Ok(());
  }

  let status = response.status();
  let body = response
    .text()
    .await
    .unwrap_or_else(|_| "unknown error".to_string());

  Err(format!("[{}] HTTP {} - Response: {}", table, status, body))
}

#[tauri::command]
pub async fn save_supabase_config(
  app: AppHandle,
  url: String,
  anon_key: String,
) -> Result<(), String> {
  let normalized_url = url.trim().trim_end_matches('/').to_string();
  if normalized_url.is_empty() {
    return Err("Supabase URL is required".to_string());
  }

  let machine_id = get_or_create_machine_id(&app)?;
  let encrypted_key = encrypt_value(anon_key.trim(), &machine_id)?;
  let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;

  store.set(SUPABASE_URL_KEY, json!(normalized_url));
  store.set(SUPABASE_ANON_KEY_KEY, json!(encrypted_key));
  store.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_supabase_connection(
  app: AppHandle,
  url: String,
  anon_key: String,
) -> Result<bool, String> {
  let machine_id = get_or_create_machine_id(&app)?;
  let endpoint = build_rest_url(
    url.trim().trim_end_matches('/'),
    "wf_patients",
    &[("limit", "1".to_string())],
  )?;
  let response = with_auth(
    supabase_client().get(endpoint),
    anon_key.trim(),
    &machine_id,
  )
  .send()
  .await
  .map_err(|e| e.to_string())?;
  Ok(response.status().is_success())
}

#[tauri::command]
pub async fn get_sync_summary(app: AppHandle) -> Result<SyncSummary, String> {
  let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
  Ok(SyncSummary {
    has_anon_key: store
      .get(SUPABASE_ANON_KEY_KEY)
      .and_then(|value| value.as_str().map(str::to_owned))
      .is_some(),
    supabase_url: store
      .get(SUPABASE_URL_KEY)
      .and_then(|value| value.as_str().map(str::to_owned)),
  })
}

#[tauri::command]
pub async fn push_to_supabase(
  app: AppHandle,
  state: State<'_, AppState>,
) -> Result<SyncResult, String> {
  let (url, anon_key) = get_supabase_config(&app)?;
  let machine_id = get_or_create_machine_id(&app)?;
  let client = supabase_client();
  let now = chrono::Utc::now().to_rfc3339();
  let mut result = SyncResult::default();

  ensure_sync_ids(&state.pool, "wf_patients", &machine_id).await?;
  ensure_sync_ids(&state.pool, "wf_visits", &machine_id).await?;
  ensure_sync_ids(&state.pool, "wf_dose_history", &machine_id).await?;
  ensure_sync_ids(&state.pool, "wf_appointments", &machine_id).await?;
  ensure_sync_ids(&state.pool, "wf_outcomes", &machine_id).await?;
  ensure_sync_ids(&state.pool, "wf_patient_status_history", &machine_id).await?;

  let patient_rows: Vec<WfPatientSync> = sqlx::query_as(
    "SELECT sync_id, machine_id, hn, enrolled_at, enrolled_by, status, indication, \
            target_inr_low, target_inr_high, notes, created_at, updated_at, deleted_at \
       FROM wf_patients \
      WHERE sync_id IS NOT NULL AND (synced_at IS NULL OR updated_at > synced_at)",
  )
  .fetch_all(&state.pool)
  .await
  .map_err(|e| e.to_string())?;
  if let Err(error) = push_rows(
    &client,
    &url,
    &anon_key,
    &machine_id,
    "wf_patients",
    "sync_id",
    &patient_rows,
  )
  .await
  {
    result.errors.push(format!("wf_patients: {error}"));
  } else {
    let sync_ids = sync_ids_from_rows(&patient_rows, |row| row.sync_id.as_ref());
    mark_rows_synced(&state.pool, "wf_patients", &sync_ids, &now).await?;
    result.pushed += patient_rows.len();
  }

  let visit_rows: Vec<WfVisitSync> = sqlx::query_as(
    "SELECT sync_id, machine_id, hn, visit_date, inr_value, inr_source, current_dose_mgday, \
            dose_detail, new_dose_mgday, new_dose_detail, new_dose_description, \
            selected_dose_option, dose_changed, next_appointment, next_inr_due, physician, \
            notes, side_effects, adherence, created_by, reviewed_at, reviewed_by, \
            created_at, updated_at, deleted_at \
       FROM wf_visits \
      WHERE sync_id IS NOT NULL AND (synced_at IS NULL OR updated_at > synced_at)",
  )
  .fetch_all(&state.pool)
  .await
  .map_err(|e| e.to_string())?;
  if let Err(error) = push_rows(
    &client,
    &url,
    &anon_key,
    &machine_id,
    "wf_visits",
    "sync_id",
    &visit_rows,
  )
  .await
  {
    result.errors.push(format!("wf_visits: {error}"));
  } else {
    let sync_ids = sync_ids_from_rows(&visit_rows, |row| row.sync_id.as_ref());
    mark_rows_synced(&state.pool, "wf_visits", &sync_ids, &now).await?;
    result.pushed += visit_rows.len();
  }

  let dose_history_rows: Vec<WfDoseHistorySync> = sqlx::query_as(
    "SELECT sync_id, machine_id, hn, changed_at, old_dose_mgday, new_dose_mgday, old_detail, \
            new_detail, reason, inr_at_change, changed_by, created_at, updated_at, deleted_at \
       FROM wf_dose_history \
      WHERE sync_id IS NOT NULL AND (synced_at IS NULL OR updated_at > synced_at)",
  )
  .fetch_all(&state.pool)
  .await
  .map_err(|e| e.to_string())?;
  if let Err(error) = push_rows(
    &client,
    &url,
    &anon_key,
    &machine_id,
    "wf_dose_history",
    "sync_id",
    &dose_history_rows,
  )
  .await
  {
    result.errors.push(format!("wf_dose_history: {error}"));
  } else {
    let sync_ids = sync_ids_from_rows(&dose_history_rows, |row| row.sync_id.as_ref());
    mark_rows_synced(&state.pool, "wf_dose_history", &sync_ids, &now).await?;
    result.pushed += dose_history_rows.len();
  }

  let appointment_rows: Vec<WfAppointmentSync> = sqlx::query_as(
    "SELECT a.sync_id, a.machine_id, a.hn, a.appt_date, a.appt_type, a.status, a.notes, \
            a.source_visit_id, COALESCE(a.source_visit_sync_id, v.sync_id) AS source_visit_sync_id, \
            a.generated_from_visit, a.created_at, a.updated_at, a.deleted_at \
       FROM wf_appointments a \
       LEFT JOIN wf_visits v ON v.id = a.source_visit_id \
      WHERE a.sync_id IS NOT NULL AND (a.synced_at IS NULL OR a.updated_at > a.synced_at)",
  )
  .fetch_all(&state.pool)
  .await
  .map_err(|e| e.to_string())?;
  if let Err(error) = push_rows(
    &client,
    &url,
    &anon_key,
    &machine_id,
    "wf_appointments",
    "sync_id",
    &appointment_rows,
  )
  .await
  {
    result.errors.push(format!("wf_appointments: {error}"));
  } else {
    let sync_ids = sync_ids_from_rows(&appointment_rows, |row| row.sync_id.as_ref());
    mark_rows_synced(&state.pool, "wf_appointments", &sync_ids, &now).await?;
    result.pushed += appointment_rows.len();
  }

  let outcome_rows: Vec<WfOutcomeSync> = sqlx::query_as(
    "SELECT sync_id, machine_id, hn, event_date, event_type, description, inr_at_event, \
            action_taken, created_by, created_at, updated_at, deleted_at \
       FROM wf_outcomes \
      WHERE sync_id IS NOT NULL AND (synced_at IS NULL OR updated_at > synced_at)",
  )
  .fetch_all(&state.pool)
  .await
  .map_err(|e| e.to_string())?;
  if let Err(error) = push_rows(
    &client,
    &url,
    &anon_key,
    &machine_id,
    "wf_outcomes",
    "sync_id",
    &outcome_rows,
  )
  .await
  {
    result.errors.push(format!("wf_outcomes: {error}"));
  } else {
    let sync_ids = sync_ids_from_rows(&outcome_rows, |row| row.sync_id.as_ref());
    mark_rows_synced(&state.pool, "wf_outcomes", &sync_ids, &now).await?;
    result.pushed += outcome_rows.len();
  }

  let history_rows: Vec<WfPatientStatusHistorySync> = sqlx::query_as(
    "SELECT sync_id, machine_id, hn, status, reason, effective_date, created_at, updated_at, deleted_at \
       FROM wf_patient_status_history \
      WHERE sync_id IS NOT NULL AND (synced_at IS NULL OR updated_at > synced_at)",
  )
  .fetch_all(&state.pool)
  .await
  .map_err(|e| e.to_string())?;
  if let Err(error) = push_rows(
    &client,
    &url,
    &anon_key,
    &machine_id,
    "wf_patient_status_history",
    "sync_id",
    &history_rows,
  )
  .await
  {
    result
      .errors
      .push(format!("wf_patient_status_history: {error}"));
  } else {
    let sync_ids = sync_ids_from_rows(&history_rows, |row| row.sync_id.as_ref());
    mark_rows_synced(&state.pool, "wf_patient_status_history", &sync_ids, &now).await?;
    result.pushed += history_rows.len();
  }

  let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
  store.set(LAST_SYNC_AT_KEY, json!(now));
  store.save().map_err(|e| e.to_string())?;

  Ok(result)
}

#[tauri::command]
pub async fn pull_from_supabase(
  app: AppHandle,
  state: State<'_, AppState>,
) -> Result<SyncResult, String> {
  let (url, anon_key) = get_supabase_config(&app)?;
  let machine_id = get_or_create_machine_id(&app)?;
  let client = supabase_client();
  let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
  let last_pull_at = store
    .get(LAST_PULL_AT_KEY)
    .and_then(|value| value.as_str().map(str::to_owned))
    .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

  let mut result = SyncResult::default();

  let patient_url = build_rest_url(
    &url,
    "wf_patients",
    &[
      ("updated_at", format!("gt.{}", last_pull_at)),
      ("order", "updated_at.asc,sync_id.asc".to_string()),
    ],
  )?;
  let patient_response = with_auth(client.get(patient_url.clone()), &anon_key, &machine_id)
    .send()
    .await
    .map_err(|e| format!("[wf_patients] Network error: {} | URL: {}", e, patient_url))?;

  if !patient_response.status().is_success() {
    let status = patient_response.status();
    let body = patient_response.text().await.unwrap_or_default();
    return Err(format!(
      "[wf_patients] HTTP {} - Response: {}\nQuery URL: {}",
      status, body, patient_url
    ));
  }

  let patient_rows: Vec<WfPatientSync> = patient_response.json().await.map_err(|e| {
    format!(
      "[wf_patients] JSON parse error: {} - Response may be empty or malformed",
      e
    )
  })?;

  for row in &patient_rows {
    let sync_id = row
      .sync_id
      .as_ref()
      .ok_or("[wf_patients] sync_id is null")?;

    let existing: Option<String> =
      sqlx::query_scalar("SELECT updated_at FROM wf_patients WHERE sync_id = ?")
        .bind(sync_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| e.to_string())?;

    let affected = if let Some(existing_updated) = existing {
      let should_update = row.updated_at > existing_updated;
      if should_update {
        sqlx::query(
          "UPDATE wf_patients SET machine_id = ?, hn = ?, enrolled_at = ?, enrolled_by = ?, \
           status = ?, indication = ?, target_inr_low = ?, target_inr_high = ?, notes = ?, \
           created_at = ?, updated_at = ?, deleted_at = ?, synced_at = ? \
           WHERE sync_id = ?",
        )
        .bind(&row.machine_id)
        .bind(&row.hn)
        .bind(&row.enrolled_at)
        .bind(&row.enrolled_by)
        .bind(&row.status)
        .bind(&row.indication)
        .bind(row.target_inr_low)
        .bind(row.target_inr_high)
        .bind(&row.notes)
        .bind(&row.created_at)
        .bind(&row.updated_at)
        .bind(&row.deleted_at)
        .bind(&row.updated_at)
        .bind(sync_id)
        .execute(&state.pool)
        .await
        .map_err(|e| e.to_string())?
        .rows_affected()
      } else {
        0
      }
    } else {
      sqlx::query(
        "INSERT INTO wf_patients \
            (sync_id, machine_id, hn, enrolled_at, enrolled_by, status, indication, target_inr_low, \
             target_inr_high, notes, created_at, updated_at, deleted_at, synced_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
      )
      .bind(sync_id)
      .bind(&row.machine_id)
      .bind(&row.hn)
      .bind(&row.enrolled_at)
      .bind(&row.enrolled_by)
      .bind(&row.status)
      .bind(&row.indication)
      .bind(row.target_inr_low)
      .bind(row.target_inr_high)
      .bind(&row.notes)
      .bind(&row.created_at)
      .bind(&row.updated_at)
      .bind(&row.deleted_at)
      .bind(&row.updated_at)
      .execute(&state.pool)
      .await
      .map_err(|e| e.to_string())?
      .rows_affected()
    };

    if affected > 0 {
      result.pulled += 1;
    } else {
      result.conflicts += 1;
    }
  }

  let visit_url = build_rest_url(
    &url,
    "wf_visits",
    &[("updated_at", format!("gt.{last_pull_at}"))],
  )?;
  let visit_response = with_auth(client.get(visit_url.clone()), &anon_key, &machine_id)
    .send()
    .await
    .map_err(|e| format!("[wf_visits] Network error: {}", e))?;

  if !visit_response.status().is_success() {
    let status = visit_response.status();
    let body = visit_response.text().await.unwrap_or_default();
    return Err(format!(
      "[wf_visits] HTTP {} - Response: {}\nQuery URL: {}",
      status, body, visit_url
    ));
  }

  let visit_rows: Vec<WfVisitSync> = visit_response
    .json()
    .await
    .map_err(|e| format!("[wf_visits] JSON parse error: {}", e))?;

  for row in &visit_rows {
    let sync_id = row.sync_id.as_ref().ok_or("[wf_visits] sync_id is null")?;

    let existing: Option<String> =
      sqlx::query_scalar("SELECT updated_at FROM wf_visits WHERE sync_id = ?")
        .bind(sync_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| e.to_string())?;

    let affected = if let Some(existing_updated) = existing {
      let should_update = row.updated_at > existing_updated;
      if should_update {
        sqlx::query(
          "UPDATE wf_visits SET machine_id = ?, hn = ?, visit_date = ?, inr_value = ?, inr_source = ?, \
           current_dose_mgday = ?, dose_detail = ?, new_dose_mgday = ?, new_dose_detail = ?, \
           new_dose_description = ?, selected_dose_option = ?, dose_changed = ?, next_appointment = ?, \
           next_inr_due = ?, physician = ?, notes = ?, side_effects = ?, adherence = ?, created_by = ?, \
           reviewed_at = ?, reviewed_by = ?, updated_at = ?, deleted_at = ?, synced_at = ? \
           WHERE sync_id = ?"
        )
        .bind(&row.machine_id)
        .bind(&row.hn)
        .bind(&row.visit_date)
        .bind(row.inr_value)
        .bind(&row.inr_source)
        .bind(row.current_dose_mgday)
        .bind(&row.dose_detail)
        .bind(row.new_dose_mgday)
        .bind(&row.new_dose_detail)
        .bind(&row.new_dose_description)
        .bind(&row.selected_dose_option)
        .bind(row.dose_changed)
        .bind(&row.next_appointment)
        .bind(&row.next_inr_due)
        .bind(&row.physician)
        .bind(&row.notes)
        .bind(&row.side_effects)
        .bind(&row.adherence)
        .bind(&row.created_by)
        .bind(&row.reviewed_at)
        .bind(&row.reviewed_by)
        .bind(&row.updated_at)
        .bind(&row.deleted_at)
        .bind(&row.updated_at)
        .bind(sync_id)
        .execute(&state.pool)
        .await
        .map_err(|e| e.to_string())?
        .rows_affected()
      } else {
        0
      }
    } else {
      sqlx::query(
        "INSERT INTO wf_visits \
            (sync_id, machine_id, hn, visit_date, inr_value, inr_source, current_dose_mgday, \
             dose_detail, new_dose_mgday, new_dose_detail, new_dose_description, selected_dose_option, \
             dose_changed, next_appointment, next_inr_due, physician, notes, side_effects, adherence, \
             created_by, reviewed_at, reviewed_by, created_at, updated_at, deleted_at, synced_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
      )
      .bind(sync_id)
      .bind(&row.machine_id)
      .bind(&row.hn)
      .bind(&row.visit_date)
      .bind(row.inr_value)
      .bind(&row.inr_source)
      .bind(row.current_dose_mgday)
      .bind(&row.dose_detail)
      .bind(row.new_dose_mgday)
      .bind(&row.new_dose_detail)
      .bind(&row.new_dose_description)
      .bind(&row.selected_dose_option)
      .bind(row.dose_changed)
      .bind(&row.next_appointment)
      .bind(&row.next_inr_due)
      .bind(&row.physician)
      .bind(&row.notes)
      .bind(&row.side_effects)
      .bind(&row.adherence)
      .bind(&row.created_by)
      .bind(&row.reviewed_at)
      .bind(&row.reviewed_by)
      .bind(&row.created_at)
      .bind(&row.updated_at)
      .bind(&row.deleted_at)
      .bind(&row.updated_at)
      .execute(&state.pool)
      .await
      .map_err(|e| e.to_string())?
      .rows_affected()
    };

    if affected > 0 {
      result.pulled += 1;
    } else {
      result.conflicts += 1;
    }
  }

  let dose_url = build_rest_url(
    &url,
    "wf_dose_history",
    &[("updated_at", format!("gt.{last_pull_at}"))],
  )?;
  let dose_response = with_auth(client.get(dose_url.clone()), &anon_key, &machine_id)
    .send()
    .await
    .map_err(|e| format!("[wf_dose_history] Network error: {}", e))?;

  if !dose_response.status().is_success() {
    let status = dose_response.status();
    let body = dose_response.text().await.unwrap_or_default();
    return Err(format!(
      "[wf_dose_history] HTTP {} - Response: {}\nQuery URL: {}",
      status, body, dose_url
    ));
  }

  let dose_rows: Vec<WfDoseHistorySync> = dose_response
    .json()
    .await
    .map_err(|e| format!("[wf_dose_history] JSON parse error: {}", e))?;
  for row in &dose_rows {
    let sync_id = row
      .sync_id
      .as_ref()
      .ok_or("[wf_dose_history] sync_id is null")?;

    let existing: Option<String> =
      sqlx::query_scalar("SELECT updated_at FROM wf_dose_history WHERE sync_id = ?")
        .bind(sync_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| e.to_string())?;

    let affected = if let Some(existing_updated) = existing {
      let should_update = row.updated_at > existing_updated;
      if should_update {
        sqlx::query(
          "UPDATE wf_dose_history SET machine_id = ?, hn = ?, changed_at = ?, old_dose_mgday = ?, \
           new_dose_mgday = ?, old_detail = ?, new_detail = ?, reason = ?, inr_at_change = ?, \
           changed_by = ?, updated_at = ?, deleted_at = ?, synced_at = ? WHERE sync_id = ?",
        )
        .bind(&row.machine_id)
        .bind(&row.hn)
        .bind(&row.changed_at)
        .bind(row.old_dose_mgday)
        .bind(row.new_dose_mgday)
        .bind(&row.old_detail)
        .bind(&row.new_detail)
        .bind(&row.reason)
        .bind(row.inr_at_change)
        .bind(&row.changed_by)
        .bind(&row.updated_at)
        .bind(&row.deleted_at)
        .bind(&row.updated_at)
        .bind(sync_id)
        .execute(&state.pool)
        .await
        .map_err(|e| e.to_string())?
        .rows_affected()
      } else {
        0
      }
    } else {
      sqlx::query(
        "INSERT INTO wf_dose_history \
            (sync_id, machine_id, hn, changed_at, old_dose_mgday, new_dose_mgday, old_detail, \
             new_detail, reason, inr_at_change, changed_by, created_at, updated_at, deleted_at, synced_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
      )
      .bind(sync_id)
      .bind(&row.machine_id)
      .bind(&row.hn)
      .bind(&row.changed_at)
      .bind(row.old_dose_mgday)
      .bind(row.new_dose_mgday)
      .bind(&row.old_detail)
      .bind(&row.new_detail)
      .bind(&row.reason)
      .bind(row.inr_at_change)
      .bind(&row.changed_by)
      .bind(&row.created_at)
      .bind(&row.updated_at)
      .bind(&row.deleted_at)
      .bind(&row.updated_at)
      .execute(&state.pool)
      .await
      .map_err(|e| e.to_string())?
      .rows_affected()
    };

    if affected > 0 {
      result.pulled += 1;
    } else {
      result.conflicts += 1;
    }
  }

  let appointment_url = build_rest_url(
    &url,
    "wf_appointments",
    &[("updated_at", format!("gt.{last_pull_at}"))],
  )?;
  let appointment_response = with_auth(client.get(appointment_url.clone()), &anon_key, &machine_id)
    .send()
    .await
    .map_err(|e| format!("[wf_appointments] Network error: {}", e))?;

  if !appointment_response.status().is_success() {
    let status = appointment_response.status();
    let body = appointment_response.text().await.unwrap_or_default();
    return Err(format!(
      "[wf_appointments] HTTP {} - Response: {}\nQuery URL: {}",
      status, body, appointment_url
    ));
  }

  let appointment_rows: Vec<WfAppointmentSync> = appointment_response
    .json()
    .await
    .map_err(|e| format!("[wf_appointments] JSON parse error: {}", e))?;
  for row in &appointment_rows {
    let sync_id = row
      .sync_id
      .as_ref()
      .ok_or("[wf_appointments] sync_id is null")?;

    let existing: Option<String> =
      sqlx::query_scalar("SELECT updated_at FROM wf_appointments WHERE sync_id = ?")
        .bind(sync_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| e.to_string())?;

    let resolved_source_visit_id = resolve_local_visit_id(
      &state.pool,
      row.source_visit_sync_id.as_deref(),
    )
    .await?;
    let source_visit_id = resolved_source_visit_id.or(row.source_visit_id);

    let affected = if let Some(existing_updated) = existing {
      let should_update = row.updated_at > existing_updated;
      if should_update {
        sqlx::query(
          "UPDATE wf_appointments SET machine_id = ?, hn = ?, appt_date = ?, appt_type = ?, \
           status = ?, notes = ?, source_visit_id = ?, source_visit_sync_id = ?, generated_from_visit = ?, \
           updated_at = ?, deleted_at = ?, synced_at = ? WHERE sync_id = ?",
        )
        .bind(&row.machine_id)
        .bind(&row.hn)
        .bind(&row.appt_date)
        .bind(&row.appt_type)
        .bind(&row.status)
        .bind(&row.notes)
        .bind(source_visit_id)
        .bind(&row.source_visit_sync_id)
        .bind(row.generated_from_visit)
        .bind(&row.updated_at)
        .bind(&row.deleted_at)
        .bind(&row.updated_at)
        .bind(sync_id)
        .execute(&state.pool)
        .await
        .map_err(|e| e.to_string())?
        .rows_affected()
      } else {
        0
      }
    } else {
      sqlx::query(
        "INSERT INTO wf_appointments \
          (sync_id, machine_id, hn, appt_date, appt_type, status, notes, source_visit_id, \
           source_visit_sync_id, generated_from_visit, created_at, updated_at, deleted_at, synced_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
      )
      .bind(sync_id)
      .bind(&row.machine_id)
      .bind(&row.hn)
      .bind(&row.appt_date)
      .bind(&row.appt_type)
      .bind(&row.status)
      .bind(&row.notes)
        .bind(source_visit_id)
        .bind(&row.source_visit_sync_id)
      .bind(row.generated_from_visit)
      .bind(&row.created_at)
      .bind(&row.updated_at)
      .bind(&row.deleted_at)
      .bind(&row.updated_at)
      .execute(&state.pool)
      .await
      .map_err(|e| e.to_string())?
      .rows_affected()
    };

    if affected > 0 {
      result.pulled += 1;
    } else {
      result.conflicts += 1;
    }
  }

  let outcome_url = build_rest_url(
    &url,
    "wf_outcomes",
    &[("updated_at", format!("gt.{last_pull_at}"))],
  )?;
  let outcome_response = with_auth(client.get(outcome_url.clone()), &anon_key, &machine_id)
    .send()
    .await
    .map_err(|e| format!("[wf_outcomes] Network error: {}", e))?;

  if !outcome_response.status().is_success() {
    let status = outcome_response.status();
    let body = outcome_response.text().await.unwrap_or_default();
    return Err(format!(
      "[wf_outcomes] HTTP {} - Response: {}\nQuery URL: {}",
      status, body, outcome_url
    ));
  }

  let outcome_rows: Vec<WfOutcomeSync> = outcome_response
    .json()
    .await
    .map_err(|e| format!("[wf_outcomes] JSON parse error: {}", e))?;
  for row in &outcome_rows {
    let sync_id = row
      .sync_id
      .as_ref()
      .ok_or("[wf_outcomes] sync_id is null")?;

    let existing: Option<String> =
      sqlx::query_scalar("SELECT updated_at FROM wf_outcomes WHERE sync_id = ?")
        .bind(sync_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| e.to_string())?;

    let affected = if let Some(existing_updated) = existing {
      let should_update = row.updated_at > existing_updated;
      if should_update {
        sqlx::query(
          "UPDATE wf_outcomes SET machine_id = ?, hn = ?, event_date = ?, event_type = ?, \
           description = ?, inr_at_event = ?, action_taken = ?, created_by = ?, \
           updated_at = ?, deleted_at = ?, synced_at = ? WHERE sync_id = ?",
        )
        .bind(&row.machine_id)
        .bind(&row.hn)
        .bind(&row.event_date)
        .bind(&row.event_type)
        .bind(&row.description)
        .bind(row.inr_at_event)
        .bind(&row.action_taken)
        .bind(&row.created_by)
        .bind(&row.updated_at)
        .bind(&row.deleted_at)
        .bind(&row.updated_at)
        .bind(sync_id)
        .execute(&state.pool)
        .await
        .map_err(|e| e.to_string())?
        .rows_affected()
      } else {
        0
      }
    } else {
      sqlx::query(
        "INSERT INTO wf_outcomes \
            (sync_id, machine_id, hn, event_date, event_type, description, inr_at_event, action_taken, \
             created_by, created_at, updated_at, deleted_at, synced_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
      )
      .bind(sync_id)
      .bind(&row.machine_id)
      .bind(&row.hn)
      .bind(&row.event_date)
      .bind(&row.event_type)
      .bind(&row.description)
      .bind(row.inr_at_event)
      .bind(&row.action_taken)
      .bind(&row.created_by)
      .bind(&row.created_at)
      .bind(&row.updated_at)
      .bind(&row.deleted_at)
      .bind(&row.updated_at)
      .execute(&state.pool)
      .await
      .map_err(|e| e.to_string())?
      .rows_affected()
    };

    if affected > 0 {
      result.pulled += 1;
    } else {
      result.conflicts += 1;
    }
  }

  let history_url = build_rest_url(
    &url,
    "wf_patient_status_history",
    &[("updated_at", format!("gt.{last_pull_at}"))],
  )?;
  let history_response = with_auth(client.get(history_url.clone()), &anon_key, &machine_id)
    .send()
    .await
    .map_err(|e| format!("[wf_patient_status_history] Network error: {}", e))?;

  if !history_response.status().is_success() {
    let status = history_response.status();
    let body = history_response.text().await.unwrap_or_default();
    return Err(format!(
      "[wf_patient_status_history] HTTP {} - Response: {}\nQuery URL: {}",
      status, body, history_url
    ));
  }

  let history_rows: Vec<WfPatientStatusHistorySync> = history_response
    .json()
    .await
    .map_err(|e| format!("[wf_patient_status_history] JSON parse error: {}", e))?;
  for row in &history_rows {
    let sync_id = row
      .sync_id
      .as_ref()
      .ok_or("[wf_patient_status_history] sync_id is null")?;

    let existing: Option<String> =
      sqlx::query_scalar("SELECT updated_at FROM wf_patient_status_history WHERE sync_id = ?")
        .bind(sync_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| e.to_string())?;

    let affected = if let Some(existing_updated) = existing {
      let should_update = row.updated_at > existing_updated;
      if should_update {
        sqlx::query(
          "UPDATE wf_patient_status_history SET machine_id = ?, hn = ?, status = ?, reason = ?, \
           effective_date = ?, updated_at = ?, deleted_at = ?, synced_at = ? WHERE sync_id = ?",
        )
        .bind(&row.machine_id)
        .bind(&row.hn)
        .bind(&row.status)
        .bind(&row.reason)
        .bind(&row.effective_date)
        .bind(&row.updated_at)
        .bind(&row.deleted_at)
        .bind(&row.updated_at)
        .bind(sync_id)
        .execute(&state.pool)
        .await
        .map_err(|e| e.to_string())?
        .rows_affected()
      } else {
        0
      }
    } else {
      sqlx::query(
        "INSERT INTO wf_patient_status_history \
            (sync_id, machine_id, hn, status, reason, effective_date, created_at, updated_at, deleted_at, synced_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
      )
      .bind(sync_id)
      .bind(&row.machine_id)
      .bind(&row.hn)
      .bind(&row.status)
      .bind(&row.reason)
      .bind(&row.effective_date)
      .bind(&row.created_at)
      .bind(&row.updated_at)
      .bind(&row.deleted_at)
      .bind(&row.updated_at)
      .execute(&state.pool)
      .await
      .map_err(|e| e.to_string())?
      .rows_affected()
    };

    if affected > 0 {
      result.pulled += 1;
    } else {
      result.conflicts += 1;
    }
  }

  let pulled_at = chrono::Utc::now().to_rfc3339();
  store.set(LAST_PULL_AT_KEY, json!(pulled_at.clone()));
  store.set(LAST_SYNC_AT_KEY, json!(pulled_at));
  store.save().map_err(|e| e.to_string())?;

  Ok(result)
}

#[tauri::command]
pub async fn get_sync_status(
  app: AppHandle,
  state: State<'_, AppState>,
) -> Result<SyncStatus, String> {
  let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
  let machine_id = get_or_create_machine_id(&app)?;

  let pending_count = sqlx::query_scalar::<_, i64>(
    "SELECT \
        (SELECT COUNT(*) FROM wf_patients WHERE synced_at IS NULL OR updated_at > synced_at) + \
        (SELECT COUNT(*) FROM wf_visits WHERE synced_at IS NULL OR updated_at > synced_at) + \
        (SELECT COUNT(*) FROM wf_dose_history WHERE synced_at IS NULL OR updated_at > synced_at) + \
        (SELECT COUNT(*) FROM wf_appointments WHERE synced_at IS NULL OR updated_at > synced_at) + \
        (SELECT COUNT(*) FROM wf_outcomes WHERE synced_at IS NULL OR updated_at > synced_at) + \
        (SELECT COUNT(*) FROM wf_patient_status_history WHERE synced_at IS NULL OR updated_at > synced_at)",
  )
  .fetch_one(&state.pool)
  .await
  .context("failed to calculate sync status")
  .map_err(|e| e.to_string())?;

  Ok(SyncStatus {
    pending_count,
    last_sync_at: store
      .get(LAST_SYNC_AT_KEY)
      .and_then(|value| value.as_str().map(str::to_owned)),
    configured: get_supabase_config(&app).is_ok(),
    machine_id,
  })
}
