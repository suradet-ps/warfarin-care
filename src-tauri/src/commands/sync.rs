use anyhow::Context;
use reqwest::{Client, Url};
use serde_json::json;
use sqlx::{QueryBuilder, Sqlite};
use std::pin::Pin;
use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;
use uuid::Uuid;

use warfarin_core::encrypt::{decrypt_value, encrypt_value};
use warfarin_db::sqlite::AppState;
use warfarin_db::sync_models::{
  ConnectionTestResult, PulledRow, SyncResult, SyncStatus, SyncSummary, WfAppointmentSync,
  WfDoseHistorySync, WfOutcomeSync, WfPatientStatusHistorySync, WfPatientSync, WfVisitSync,
};

/// Alias for an async block that performs a `SQLite` write and returns either
/// the number of rows touched or an error string. Used by `pull_table` to
/// accept per-table INSERT/UPDATE logic without an `async` closure (which
/// Rust does not yet support natively).
type SqliteFut<'a, T> = Pin<Box<dyn std::future::Future<Output = Result<T, String>> + Send + 'a>>;

/// Pulls rows for a single table from Supabase and applies them to the local
/// `SQLite` database, choosing INSERT vs UPDATE per row based on the
/// batch-fetched existence map. The `apply` closure encapsulates the
/// per-table SQL: it receives the existing local `updated_at` (or `None` if
/// the row is new) and returns the number of rows affected. The closure is
/// also responsible for the "stale row" check — returning `Ok(0)` when the
/// remote row is older than the local copy counts as a conflict.
#[allow(clippy::too_many_arguments)]
async fn pull_table<T, F>(
  pool: &sqlx::SqlitePool,
  client: &Client,
  base_url: &str,
  anon_key: &str,
  machine_id: &str,
  last_pull_at: &str,
  table: &str,
  apply: F,
) -> Result<(usize, usize), String>
where
  T: PulledRow + serde::de::DeserializeOwned,
  F: for<'a> Fn(&'a sqlx::SqlitePool, &'a T, Option<&'a str>) -> SqliteFut<'a, u64>,
{
  assert_table_allowed(table)?;

  let url = build_rest_url(
    base_url,
    table,
    &[("updated_at", format!("gt.{last_pull_at}"))],
  )?;
  let response = with_auth(client.get(url.clone()), anon_key, machine_id)
    .send()
    .await
    .map_err(|e| format!("[{table}] Network error: {e} | URL: {url}"))?;
  if !response.status().is_success() {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    return Err(format!(
      "[{table}] HTTP {status} - Response: {body}\nQuery URL: {url}"
    ));
  }
  let rows: Vec<T> = response
    .json()
    .await
    .map_err(|e| format!("[{table}] JSON parse error: {e} - Response may be empty or malformed"))?;

  let sync_ids: Vec<String> = rows.iter().filter_map(T::sync_id).cloned().collect();
  let existing = fetch_existing_updated_ats(pool, table, &sync_ids).await?;

  let mut pulled = 0usize;
  let mut conflicts = 0usize;
  for row in &rows {
    let sync_id = T::sync_id(row).ok_or_else(|| format!("[{table}] sync_id is null"))?;
    let prev_updated = existing.get(sync_id).map(String::as_str);
    let affected = apply(pool, row, prev_updated).await?;
    if affected > 0 {
      pulled += 1;
    } else {
      conflicts += 1;
    }
  }
  Ok((pulled, conflicts))
}

const STORE_FILE: &str = "config.json";
const SUPABASE_URL_KEY: &str = "supabase_url";
const SUPABASE_ANON_KEY_KEY: &str = "supabase_anon_key_enc";
const MACHINE_ID_KEY: &str = "machine_id";
const LAST_PULL_AT_KEY: &str = "last_pull_at";
const LAST_SYNC_AT_KEY: &str = "last_sync_at";

/// Allowlist of table names that can be passed into `ensure_sync_ids` and other
/// dynamic-table SQL builders. The Rust compiler cannot verify that the table
/// parameter is one of these — we match on it explicitly in every helper that
/// takes `table: &str`.
const SYNC_TABLES: &[&str] = &[
  "wf_patients",
  "wf_visits",
  "wf_dose_history",
  "wf_appointments",
  "wf_outcomes",
  "wf_patient_status_history",
];

fn assert_table_allowed(table: &str) -> Result<(), String> {
  if SYNC_TABLES.contains(&table) {
    Ok(())
  } else {
    Err(format!("disallowed table name '{table}'"))
  }
}

/// Supabase REST API requires HTTPS. Reject `http://` to avoid sending
/// bearer tokens and patient data in cleartext. `localhost` is allowed
/// only when the host is literally `localhost` or `127.0.0.1` (useful
/// during local development against a self-hosted `PostgREST`).
fn ensure_https(url: &str) -> Result<(), String> {
  let parsed = Url::parse(url).map_err(|e| format!("invalid Supabase URL: {e}"))?;
  match parsed.scheme() {
    "https" => Ok(()),
    "http" if matches!(parsed.host_str(), Some("localhost" | "127.0.0.1")) => Ok(()),
    "http" => Err("Supabase URL must use HTTPS in production".to_string()),
    other => Err(format!("unsupported Supabase URL scheme '{other}'")),
  }
}

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
  ensure_https(&url)?;

  let encrypted_key = store
    .get(SUPABASE_ANON_KEY_KEY)
    .and_then(|value| value.as_str().map(str::to_owned))
    .ok_or_else(|| "Supabase anon key is not configured".to_string())?;

  let anon_key = decrypt_value(&encrypted_key, &machine_id)?;
  Ok((url, anon_key))
}

fn supabase_client() -> Client {
  Client::builder()
    .timeout(std::time::Duration::from_secs(30))
    .connect_timeout(std::time::Duration::from_secs(10))
    .user_agent(concat!("warfarin-care/", env!("CARGO_PKG_VERSION")))
    .build()
    .unwrap_or_else(|_| Client::new())
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
  assert_table_allowed(table)?;

  let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
  let mut select =
    QueryBuilder::<Sqlite>::new(format!("SELECT id FROM {table} WHERE sync_id IS NULL"));
  let ids: Vec<i64> = select
    .build_query_scalar::<i64>()
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

  for row_id in ids {
    let new_sync_id = Uuid::new_v4().to_string();
    let mut update = QueryBuilder::<Sqlite>::new(format!("UPDATE {table} SET sync_id = "));
    update.push_bind(new_sync_id);
    update.push(", machine_id = COALESCE(machine_id, ");
    update.push_bind(machine_id);
    update.push(") WHERE id = ");
    update.push_bind(row_id);
    update
      .build()
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

/// Single batched lookup that replaces the N+1 `SELECT updated_at ... WHERE
/// sync_id = ?` round-trips the original pull loop performed per row. Returns
/// a map of `sync_id -> existing updated_at` so the caller can decide per
/// row whether to INSERT or UPDATE.
async fn fetch_existing_updated_ats(
  pool: &sqlx::SqlitePool,
  table: &str,
  sync_ids: &[String],
) -> Result<std::collections::HashMap<String, String>, String> {
  assert_table_allowed(table)?;
  if sync_ids.is_empty() {
    return Ok(std::collections::HashMap::new());
  }
  let mut builder = QueryBuilder::<Sqlite>::new(format!(
    "SELECT sync_id, updated_at FROM {table} WHERE sync_id IN ("
  ));
  {
    let mut separated = builder.separated(", ");
    for sync_id in sync_ids {
      separated.push_bind(sync_id);
    }
  }
  builder.push(")");
  let rows: Vec<(String, String)> = builder
    .build_query_as::<(String, String)>()
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
  Ok(rows.into_iter().collect())
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
    .map_err(|e| format!("[{table}] Network error: {e}"))?;

  if response.status().is_success() {
    return Ok(());
  }

  let status = response.status();
  let body = response
    .text()
    .await
    .unwrap_or_else(|_| "unknown error".to_string());

  Err(format!("[{table}] HTTP {status} - Response: {body}"))
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
  ensure_https(&normalized_url)?;

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
) -> Result<ConnectionTestResult, String> {
  let trimmed_url = url.trim().trim_end_matches('/').to_string();
  if trimmed_url.is_empty() {
    return Err("กรุณากรอก Supabase URL".to_string());
  }
  let trimmed_key = anon_key.trim();
  if trimmed_key.is_empty() {
    return Err("กรุณากรอก Anon Key".to_string());
  }
  ensure_https(&trimmed_url)?;

  let machine_id = get_or_create_machine_id(&app)?;
  let client = supabase_client();

  // Probe `wf_patients?limit=1` directly. Supabase's `/rest/v1/` root is
  // service_role-only, so it always rejects anon keys with 401 — using it
  // as a "connection test" gave false negatives. The table probe works with
  // anon keys when RLS allows SELECT, and PostgREST returns a structured
  // JSON error body whose `code` / `message` we can use to tell the user
  // precisely what's wrong (bad key vs missing table vs RLS block).
  let endpoint = build_rest_url(&trimmed_url, "wf_patients", &[("limit", "1".to_string())])?;
  let response = with_auth(client.get(endpoint.clone()), trimmed_key, &machine_id)
    .send()
    .await
    .map_err(|e| format!("ไม่สามารถเชื่อมต่อ {endpoint} ได้: {e} (ตรวจสอบ URL และเครือข่าย)"))?;

  let status = response.status();
  let status_code = Some(status.as_u16());
  let body = response.text().await.unwrap_or_default();
  let preview: String = body.chars().take(300).collect();

  if status.is_success() {
    return Ok(ConnectionTestResult {
      ok: true,
      message: "เชื่อมต่อสำเร็จ พร้อมใช้งาน Push/Pull".to_string(),
      status_code,
    });
  }

  // PostgREST returns `42P01` when the relation doesn't exist. Treat this as
  // a "connected but not yet set up" signal — distinct from a bad anon key.
  if status.as_u16() == 404 || body.contains("42P01") || body.contains("does not exist") {
    return Ok(ConnectionTestResult {
      ok: false,
      message: format!(
        "เชื่อมต่อ Supabase สำเร็จ แต่ยังไม่มีตาราง wf_patients กรุณารัน SQL setup ตาม CLOUD-SYNC.md ก่อนใช้งาน Push/Pull (HTTP {}). รายละเอียด: {}",
        status.as_u16(),
        preview
      ),
      status_code,
    });
  }

  if status.as_u16() == 401 {
    return Ok(ConnectionTestResult {
      ok: false,
      message: format!("Anon Key ไม่ถูกต้อง (HTTP 401). รายละเอียด: {preview}"),
      status_code,
    });
  }

  if status.as_u16() == 403 {
    return Ok(ConnectionTestResult {
      ok: false,
      message: format!(
        "ไม่มีสิทธิ์เข้าถึงตาราง wf_patients — ตรวจสอบ RLS policy ใน Supabase (HTTP 403). รายละเอียด: {preview}"
      ),
      status_code,
    });
  }

  Ok(ConnectionTestResult {
    ok: false,
    message: format!("เชื่อมต่อไม่สำเร็จ (HTTP {}): {}", status.as_u16(), preview),
    status_code,
  })
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

  // The first table needs an explicit `order` so the rest of the loop can
  // rely on append-only ordering; the helper handles URL building, JSON
  // parsing, batched existence check, and pull/conflict accounting for
  // every table uniformly.
  let (patients_pulled, patients_conflicts) = pull_table_patients(
    &state.pool,
    &client,
    &url,
    &anon_key,
    &machine_id,
    &last_pull_at,
  )
  .await?;
  result.pulled += patients_pulled;
  result.conflicts += patients_conflicts;

  let (visits_pulled, visits_conflicts) = pull_table::<WfVisitSync, _>(
    &state.pool,
    &client,
    &url,
    &anon_key,
    &machine_id,
    &last_pull_at,
    "wf_visits",
    |pool, row, prev| {
      Box::pin(async move {
        let row_updated = row.updated_at.as_str();
        match prev {
          None => insert_visit(pool, row).await,
          Some(prev_updated) if row_updated > prev_updated => update_visit(pool, row).await,
          Some(_) => Ok(0),
        }
      })
    },
  )
  .await?;
  result.pulled += visits_pulled;
  result.conflicts += visits_conflicts;

  let (dose_pulled, dose_conflicts) = pull_table::<WfDoseHistorySync, _>(
    &state.pool,
    &client,
    &url,
    &anon_key,
    &machine_id,
    &last_pull_at,
    "wf_dose_history",
    |pool, row, prev| {
      Box::pin(async move {
        let row_updated = row.updated_at.as_str();
        match prev {
          None => insert_dose_history(pool, row).await,
          Some(prev_updated) if row_updated > prev_updated => update_dose_history(pool, row).await,
          Some(_) => Ok(0),
        }
      })
    },
  )
  .await?;
  result.pulled += dose_pulled;
  result.conflicts += dose_conflicts;

  let (appt_pulled, appt_conflicts) = pull_table::<WfAppointmentSync, _>(
    &state.pool,
    &client,
    &url,
    &anon_key,
    &machine_id,
    &last_pull_at,
    "wf_appointments",
    |pool, row, prev| {
      Box::pin(async move {
        let row_updated = row.updated_at.as_str();
        let resolved = resolve_local_visit_id(pool, row.source_visit_sync_id.as_deref()).await?;
        let source_visit_id = resolved.or(row.source_visit_id);
        match prev {
          None => insert_appointment(pool, row, source_visit_id).await,
          Some(prev_updated) if row_updated > prev_updated => {
            update_appointment(pool, row, source_visit_id).await
          }
          Some(_) => Ok(0),
        }
      })
    },
  )
  .await?;
  result.pulled += appt_pulled;
  result.conflicts += appt_conflicts;

  let (outcome_pulled, outcome_conflicts) = pull_table::<WfOutcomeSync, _>(
    &state.pool,
    &client,
    &url,
    &anon_key,
    &machine_id,
    &last_pull_at,
    "wf_outcomes",
    |pool, row, prev| {
      Box::pin(async move {
        let row_updated = row.updated_at.as_str();
        match prev {
          None => insert_outcome(pool, row).await,
          Some(prev_updated) if row_updated > prev_updated => update_outcome(pool, row).await,
          Some(_) => Ok(0),
        }
      })
    },
  )
  .await?;
  result.pulled += outcome_pulled;
  result.conflicts += outcome_conflicts;

  let (history_pulled, history_conflicts) = pull_table::<WfPatientStatusHistorySync, _>(
    &state.pool,
    &client,
    &url,
    &anon_key,
    &machine_id,
    &last_pull_at,
    "wf_patient_status_history",
    |pool, row, prev| {
      Box::pin(async move {
        let row_updated = row.updated_at.as_str();
        match prev {
          None => insert_status_history(pool, row).await,
          Some(prev_updated) if row_updated > prev_updated => {
            update_status_history(pool, row).await
          }
          Some(_) => Ok(0),
        }
      })
    },
  )
  .await?;
  result.pulled += history_pulled;
  result.conflicts += history_conflicts;

  let pulled_at = chrono::Utc::now().to_rfc3339();
  store.set(LAST_PULL_AT_KEY, json!(pulled_at.clone()));
  store.set(LAST_SYNC_AT_KEY, json!(pulled_at));
  store.save().map_err(|e| e.to_string())?;

  Ok(result)
}

/// `wf_patients` is the only table that needs a custom `order` query
/// parameter to ensure append-only ordering on the server, so it gets its
/// own thin wrapper around `pull_table` to pass that extra query pair.
async fn pull_table_patients(
  pool: &sqlx::SqlitePool,
  client: &Client,
  base_url: &str,
  anon_key: &str,
  machine_id: &str,
  last_pull_at: &str,
) -> Result<(usize, usize), String> {
  let table = "wf_patients";
  let url = build_rest_url(
    base_url,
    table,
    &[
      ("updated_at", format!("gt.{last_pull_at}")),
      ("order", "updated_at.asc,sync_id.asc".to_string()),
    ],
  )?;
  let response = with_auth(client.get(url.clone()), anon_key, machine_id)
    .send()
    .await
    .map_err(|e| format!("[{table}] Network error: {e} | URL: {url}"))?;
  if !response.status().is_success() {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    return Err(format!(
      "[{table}] HTTP {status} - Response: {body}\nQuery URL: {url}"
    ));
  }
  let rows: Vec<WfPatientSync> = response
    .json()
    .await
    .map_err(|e| format!("[{table}] JSON parse error: {e} - Response may be empty or malformed"))?;
  let sync_ids: Vec<String> = rows
    .iter()
    .filter_map(WfPatientSync::sync_id)
    .cloned()
    .collect();
  let existing = fetch_existing_updated_ats(pool, table, &sync_ids).await?;
  let mut pulled = 0usize;
  let mut conflicts = 0usize;
  for row in &rows {
    let sync_id =
      WfPatientSync::sync_id(row).ok_or_else(|| format!("[{table}] sync_id is null"))?;
    let prev = existing.get(sync_id).map(String::as_str);
    let row_updated = row.updated_at.as_str();
    let affected = match prev {
      None => insert_patient(pool, row).await?,
      Some(prev_updated) if row_updated > prev_updated => update_patient(pool, row).await?,
      Some(_) => 0,
    };
    if affected > 0 {
      pulled += 1;
    } else {
      conflicts += 1;
    }
  }
  Ok((pulled, conflicts))
}

// Per-table INSERT helpers — keep the SQL column lists next to the row struct
// they reference for easy auditing when the schema changes.

async fn insert_patient(pool: &sqlx::SqlitePool, row: &WfPatientSync) -> Result<u64, String> {
  let sync_id = row.sync_id.as_deref().unwrap_or_default();
  sqlx::query(
    "INSERT INTO wf_patients \
        (sync_id, machine_id, hn, enrolled_at, enrolled_by, status, indication, target_inr_low, \
         target_inr_high, notes, created_at, updated_at, deleted_at, synced_at) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
  .execute(pool)
  .await
  .map(|r| r.rows_affected())
  .map_err(|e| e.to_string())
}

async fn update_patient(pool: &sqlx::SqlitePool, row: &WfPatientSync) -> Result<u64, String> {
  let sync_id = row.sync_id.as_deref().unwrap_or_default();
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
  .execute(pool)
  .await
  .map(|r| r.rows_affected())
  .map_err(|e| e.to_string())
}

async fn insert_visit(pool: &sqlx::SqlitePool, row: &WfVisitSync) -> Result<u64, String> {
  let sync_id = row.sync_id.as_deref().unwrap_or_default();
  sqlx::query(
    "INSERT INTO wf_visits \
        (sync_id, machine_id, hn, visit_date, inr_value, inr_source, current_dose_mgday, \
         dose_detail, new_dose_mgday, new_dose_detail, new_dose_description, selected_dose_option, \
         dose_changed, next_appointment, next_inr_due, physician, notes, side_effects, adherence, \
         created_by, reviewed_at, reviewed_by, created_at, updated_at, deleted_at, synced_at) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
  .execute(pool)
  .await
  .map(|r| r.rows_affected())
  .map_err(|e| e.to_string())
}

async fn update_visit(pool: &sqlx::SqlitePool, row: &WfVisitSync) -> Result<u64, String> {
  let sync_id = row.sync_id.as_deref().unwrap_or_default();
  sqlx::query(
    "UPDATE wf_visits SET machine_id = ?, hn = ?, visit_date = ?, inr_value = ?, inr_source = ?, \
     current_dose_mgday = ?, dose_detail = ?, new_dose_mgday = ?, new_dose_detail = ?, \
     new_dose_description = ?, selected_dose_option = ?, dose_changed = ?, next_appointment = ?, \
     next_inr_due = ?, physician = ?, notes = ?, side_effects = ?, adherence = ?, created_by = ?, \
     reviewed_at = ?, reviewed_by = ?, updated_at = ?, deleted_at = ?, synced_at = ? \
     WHERE sync_id = ?",
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
  .execute(pool)
  .await
  .map(|r| r.rows_affected())
  .map_err(|e| e.to_string())
}

async fn insert_dose_history(
  pool: &sqlx::SqlitePool,
  row: &WfDoseHistorySync,
) -> Result<u64, String> {
  let sync_id = row.sync_id.as_deref().unwrap_or_default();
  sqlx::query(
    "INSERT INTO wf_dose_history \
        (sync_id, machine_id, hn, changed_at, old_dose_mgday, new_dose_mgday, old_detail, \
         new_detail, reason, inr_at_change, changed_by, created_at, updated_at, deleted_at, synced_at) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
  .execute(pool)
  .await
  .map(|r| r.rows_affected())
  .map_err(|e| e.to_string())
}

async fn update_dose_history(
  pool: &sqlx::SqlitePool,
  row: &WfDoseHistorySync,
) -> Result<u64, String> {
  let sync_id = row.sync_id.as_deref().unwrap_or_default();
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
  .execute(pool)
  .await
  .map(|r| r.rows_affected())
  .map_err(|e| e.to_string())
}

async fn insert_appointment(
  pool: &sqlx::SqlitePool,
  row: &WfAppointmentSync,
  source_visit_id: Option<i64>,
) -> Result<u64, String> {
  let sync_id = row.sync_id.as_deref().unwrap_or_default();
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
  .execute(pool)
  .await
  .map(|r| r.rows_affected())
  .map_err(|e| e.to_string())
}

async fn update_appointment(
  pool: &sqlx::SqlitePool,
  row: &WfAppointmentSync,
  source_visit_id: Option<i64>,
) -> Result<u64, String> {
  let sync_id = row.sync_id.as_deref().unwrap_or_default();
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
  .execute(pool)
  .await
  .map(|r| r.rows_affected())
  .map_err(|e| e.to_string())
}

async fn insert_outcome(pool: &sqlx::SqlitePool, row: &WfOutcomeSync) -> Result<u64, String> {
  let sync_id = row.sync_id.as_deref().unwrap_or_default();
  sqlx::query(
    "INSERT INTO wf_outcomes \
        (sync_id, machine_id, hn, event_date, event_type, description, inr_at_event, action_taken, \
         created_by, created_at, updated_at, deleted_at, synced_at) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
  .execute(pool)
  .await
  .map(|r| r.rows_affected())
  .map_err(|e| e.to_string())
}

async fn update_outcome(pool: &sqlx::SqlitePool, row: &WfOutcomeSync) -> Result<u64, String> {
  let sync_id = row.sync_id.as_deref().unwrap_or_default();
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
  .execute(pool)
  .await
  .map(|r| r.rows_affected())
  .map_err(|e| e.to_string())
}

async fn insert_status_history(
  pool: &sqlx::SqlitePool,
  row: &WfPatientStatusHistorySync,
) -> Result<u64, String> {
  let sync_id = row.sync_id.as_deref().unwrap_or_default();
  sqlx::query(
    "INSERT INTO wf_patient_status_history \
        (sync_id, machine_id, hn, status, reason, effective_date, created_at, updated_at, deleted_at, synced_at) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
  .execute(pool)
  .await
  .map(|r| r.rows_affected())
  .map_err(|e| e.to_string())
}

async fn update_status_history(
  pool: &sqlx::SqlitePool,
  row: &WfPatientStatusHistorySync,
) -> Result<u64, String> {
  let sync_id = row.sync_id.as_deref().unwrap_or_default();
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
  .execute(pool)
  .await
  .map(|r| r.rows_affected())
  .map_err(|e| e.to_string())
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
