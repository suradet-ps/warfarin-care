//! `HOSxP` `MySQL` read-only query layer.
//!
//! **This module NEVER writes to `HOSxP`.** All functions return `anyhow::Result`
//! so command handlers can surface connection and query failures explicitly.
//!
//! Runtime queries (`sqlx::query()`) are used throughout because the `HOSxP`
//! `MySQL` server is only available at runtime, never at compile time.

use anyhow::{Context, Result, bail};
use chrono::{Duration, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{
  MySql, MySqlPool, QueryBuilder, Row,
  mysql::{MySqlPoolOptions, MySqlRow},
};
use std::{
  collections::{HashMap, HashSet},
  sync::OnceLock,
};
use tokio::sync::RwLock;

use warfarin_core::dose::usage_parser::parse_dispensing_usage;
use warfarin_core::models::{
  dispensing::DispensingRecord,
  inr::InrRecord,
  patient::{HosxpPatient, PatientDrugRecord, SearchFilters, SearchResponse},
};

/// Warfarin drug item codes.
pub const WARFARIN_ICODES: [&str; 3] = ["1600014", "1600013", "1600024"];
/// INR lab items code.
pub const INR_LAB_ITEM_CODE: &str = "751";

// ── Config ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbConfig {
  pub host: String,
  pub port: u16,
  pub database: String,
  pub username: String,
  pub password: String,
}

impl DbConfig {
  fn connection_url(&self) -> String {
    format!(
      "mysql://{}:{}@{}:{}/{}",
      self.username, self.password, self.host, self.port, self.database
    )
  }
}

// ── Connection helpers ────────────────────────────────────────────────────────

fn mysql_pool_cache() -> &'static RwLock<HashMap<String, MySqlPool>> {
  static MYSQL_POOL_CACHE: OnceLock<RwLock<HashMap<String, MySqlPool>>> = OnceLock::new();
  MYSQL_POOL_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub async fn create_pool(config: &DbConfig) -> Result<sqlx::MySqlPool> {
  let connection_url = config.connection_url();

  if let Some(pool) = mysql_pool_cache()
    .read()
    .await
    .get(&connection_url)
    .cloned()
  {
    return Ok(pool);
  }

  let pool = MySqlPoolOptions::new()
    .max_connections(3)
    .acquire_timeout(std::time::Duration::from_secs(5))
    .connect(&connection_url)
    .await
    .context("failed to connect to HOSxP MySQL")?;

  let mut cache = mysql_pool_cache().write().await;
  let cached = cache
    .entry(connection_url)
    .or_insert_with(|| pool.clone())
    .clone();
  Ok(cached)
}

/// Tests whether the given `MySQL` config can establish a connection.
/// Returns `true` on success, `false` on any error.
pub async fn test_mysql_connection(config: &DbConfig) -> bool {
  match create_pool(config).await {
    Ok(pool) => sqlx::query_scalar::<_, i32>("SELECT 1")
      .fetch_one(&pool)
      .await
      .is_ok(),
    Err(_) => false,
  }
}

fn default_search_window() -> (String, String) {
  let today = Utc::now().date_naive();
  let one_year_ago = today - Duration::days(365);
  (
    one_year_ago.format("%Y-%m-%d").to_string(),
    today.format("%Y-%m-%d").to_string(),
  )
}

fn resolve_search_window(filters: &SearchFilters) -> Result<(String, String)> {
  let (default_from, default_to) = default_search_window();

  let date_from = filters
    .date_from
    .as_deref()
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .unwrap_or(&default_from);
  let date_to = filters
    .date_to
    .as_deref()
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .unwrap_or(&default_to);

  let parsed_from = NaiveDate::parse_from_str(date_from, "%Y-%m-%d")
    .with_context(|| format!("invalid search start date: {date_from}"))?;
  let parsed_to = NaiveDate::parse_from_str(date_to, "%Y-%m-%d")
    .with_context(|| format!("invalid search end date: {date_to}"))?;

  if parsed_from > parsed_to {
    bail!("search start date must be on or before end date");
  }

  Ok((
    parsed_from.format("%Y-%m-%d").to_string(),
    parsed_to.format("%Y-%m-%d").to_string(),
  ))
}

pub fn get_optional_date_string(row: &MySqlRow, column: &str) -> Option<String> {
  row
    .try_get::<Option<NaiveDate>, _>(column)
    .ok()
    .flatten()
    .map(|value| value.format("%Y-%m-%d").to_string())
    .or_else(|| {
      row
        .try_get::<Option<NaiveDateTime>, _>(column)
        .ok()
        .flatten()
        .map(|value| value.date().format("%Y-%m-%d").to_string())
    })
    .or_else(|| {
      row
        .try_get::<Option<String>, _>(column)
        .ok()
        .flatten()
        .map(|value| value.split_whitespace().next().unwrap_or("").to_string())
    })
}

fn get_date_string(row: &MySqlRow, column: &str) -> String {
  get_optional_date_string(row, column).unwrap_or_default()
}

fn get_optional_string(row: &MySqlRow, column: &str) -> Option<String> {
  row
    .try_get::<Option<String>, _>(column)
    .ok()
    .flatten()
    .or_else(|| {
      row
        .try_get::<Option<i64>, _>(column)
        .ok()
        .flatten()
        .map(|value| value.to_string())
    })
    .or_else(|| {
      row
        .try_get::<Option<u64>, _>(column)
        .ok()
        .flatten()
        .map(|value| value.to_string())
    })
    .or_else(|| {
      row
        .try_get::<Option<i32>, _>(column)
        .ok()
        .flatten()
        .map(|value| value.to_string())
    })
    .or_else(|| {
      row
        .try_get::<Option<u32>, _>(column)
        .ok()
        .flatten()
        .map(|value| value.to_string())
    })
}

fn apply_keyword_filter(builder: &mut QueryBuilder<MySql>, keyword: &str) {
  let trimmed = keyword.trim();
  if trimmed.is_empty() {
    return;
  }

  let keyword_like = format!("%{trimmed}%");
  builder
        .push(" AND (p.hn LIKE ")
        .push_bind(keyword_like.clone())
        .push(" OR CONCAT_WS(' ', COALESCE(p.pname, ''), COALESCE(p.fname, ''), COALESCE(p.lname, '')) LIKE ")
        .push_bind(keyword_like.clone())
        .push(" OR CONCAT(COALESCE(p.pname, ''), COALESCE(p.fname, ''), COALESCE(p.lname, '')) LIKE ")
        .push_bind(keyword_like)
        .push(")");
}

fn apply_enrollment_filter(
  builder: &mut QueryBuilder<MySql>,
  enrollment_status: Option<&str>,
  enrolled_hns: &[String],
) {
  match enrollment_status {
    Some("enrolled" | "not_enrolled") if !enrolled_hns.is_empty() => {
      builder.push(match enrollment_status {
        Some("enrolled") => " AND p.hn IN (",
        _ => " AND p.hn NOT IN (",
      });
      {
        let mut separated = builder.separated(", ");
        for hn in enrolled_hns {
          separated.push_bind(hn.clone());
        }
      }
      builder.push(")");
    }
    _ => {}
  }
}

fn build_screening_query<'a>(
  select_clause: &'a str,
  date_from: &'a str,
  date_to: &'a str,
  keyword: &'a str,
  enrollment_status: Option<&'a str>,
  enrolled_hns: &'a [String],
) -> QueryBuilder<MySql> {
  let mut builder = QueryBuilder::<MySql>::new(select_clause);
  builder.push(
    r"
        FROM opitemrece o
        JOIN patient   p ON p.hn    = o.hn
        JOIN drugitems d ON d.icode = o.icode
        WHERE o.icode IN (
        ",
  );
  {
    let mut separated = builder.separated(", ");
    for icode in WARFARIN_ICODES {
      separated.push_bind(icode);
    }
  }
  builder
        .push(")")
        .push(" AND COALESCE(DATE(o.vstdate), STR_TO_DATE(o.vstdate, '%Y-%m-%d'), STR_TO_DATE(o.vstdate, '%d/%m/%Y')) BETWEEN ")
        .push_bind(date_from)
        .push(" AND ")
        .push_bind(date_to);

  apply_keyword_filter(&mut builder, keyword);
  apply_enrollment_filter(&mut builder, enrollment_status, enrolled_hns);

  builder
}

// ── Patient demographics ──────────────────────────────────────────────────────

/// Fetches basic demographics for a single patient from `HOSxP`.
pub async fn get_hosxp_patient(config: &DbConfig, hn: &str) -> Result<Option<HosxpPatient>> {
  let pool = create_pool(config).await?;
  let row = sqlx::query(
    "SELECT hn, pname, fname, lname, birthday, sex, informaddr, hometel FROM patient WHERE hn = ? LIMIT 1",
  )
  .bind(hn)
  .fetch_optional(&pool)
  .await
  .context("failed to query HOSxP patient")?;

  Ok(row.map(|r| HosxpPatient {
    hn: r.get("hn"),
    pname: r.try_get("pname").unwrap_or_default(),
    fname: r.try_get("fname").unwrap_or_default(),
    lname: r.try_get("lname").unwrap_or_default(),
    birthday: get_date_string(&r, "birthday"),
    sex: r.try_get("sex").unwrap_or_else(|_| "U".to_string()),
    addrpart: r.try_get("informaddr").ok(),
    phone: r.try_get("hometel").ok(),
  }))
}

async fn get_hosxp_patients_by_hns_with_pool(
  pool: &sqlx::MySqlPool,
  hns: &[String],
) -> Result<HashMap<String, HosxpPatient>> {
  if hns.is_empty() {
    return Ok(HashMap::new());
  }

  let mut query = QueryBuilder::<MySql>::new(
    "SELECT hn, pname, fname, lname, birthday, sex, informaddr, hometel FROM patient WHERE hn IN (",
  );
  {
    let mut separated = query.separated(", ");
    for hn in hns {
      separated.push_bind(hn);
    }
  }
  query.push(")");

  let rows = query
    .build()
    .fetch_all(pool)
    .await
    .context("failed to batch query HOSxP patients")?;

  Ok(
    rows
      .into_iter()
      .map(|r| {
        let patient = HosxpPatient {
          hn: r.get("hn"),
          pname: r.try_get("pname").unwrap_or_default(),
          fname: r.try_get("fname").unwrap_or_default(),
          lname: r.try_get("lname").unwrap_or_default(),
          birthday: get_date_string(&r, "birthday"),
          sex: r.try_get("sex").unwrap_or_else(|_| "U".to_string()),
          addrpart: r.try_get("informaddr").ok(),
          phone: r.try_get("hometel").ok(),
        };
        (patient.hn.clone(), patient)
      })
      .collect(),
  )
}

async fn get_inr_history_by_hns_with_pool(
  pool: &sqlx::MySqlPool,
  hns: &[String],
) -> Result<HashMap<String, Vec<InrRecord>>> {
  if hns.is_empty() {
    return Ok(HashMap::new());
  }

  let mut inhouse_query = QueryBuilder::<MySql>::new(
    "SELECT lh.hn, lh.lab_order_number, lh.vn, lh.order_date AS lab_date, lo.lab_order_result AS result \
     FROM lab_order lo JOIN lab_head lh ON lh.lab_order_number = lo.lab_order_number \
     WHERE lh.hn IN (",
  );
  {
    let mut separated = inhouse_query.separated(", ");
    for hn in hns {
      separated.push_bind(hn);
    }
  }
  inhouse_query.push(
    ") AND lo.lab_items_code = '751' \
     AND lo.lab_order_result IS NOT NULL \
     AND lo.lab_order_result <> '' \
     AND lo.lab_order_result REGEXP '^[0-9]' \
     ORDER BY lh.hn ASC, lh.order_date ASC",
  );

  let mut app_query = QueryBuilder::<MySql>::new(
    "SELECT ah.hn, ah.lab_app_order_number, ah.vn, ah.order_date AS lab_date, ao.lab_order_result AS result \
     FROM lab_app_order ao JOIN lab_app_head ah ON ah.lab_app_order_number = ao.lab_app_order_number \
     WHERE ah.hn IN (",
  );
  {
    let mut separated = app_query.separated(", ");
    for hn in hns {
      separated.push_bind(hn);
    }
  }
  app_query.push(
    ") AND ao.lab_items_code = '751' \
     AND ao.lab_order_result IS NOT NULL \
     AND ao.lab_order_result <> '' \
     AND ao.lab_order_result REGEXP '^[0-9]' \
     ORDER BY ah.hn ASC, ah.order_date ASC",
  );

  let inhouse_rows = inhouse_query
    .build()
    .fetch_all(pool)
    .await
    .context("failed to batch query in-house INR history")?;
  let app_rows = app_query
    .build()
    .fetch_all(pool)
    .await
    .context("failed to batch query external INR history")?;

  let mut grouped: HashMap<String, std::collections::BTreeMap<String, InrRecord>> = HashMap::new();

  for row in &app_rows {
    let hn = row.try_get::<String, _>("hn").unwrap_or_default();
    let date = match get_optional_date_string(row, "lab_date") {
      Some(d) if !d.is_empty() => d,
      _ => continue,
    };
    let result_str: String = row.try_get("result").unwrap_or_default();
    if let Ok(value) = result_str.trim().parse::<f64>() {
      grouped.entry(hn).or_default().insert(
        date.clone(),
        InrRecord {
          date,
          value,
          source: "lab_app_order".to_string(),
          lab_order_number: get_optional_string(row, "lab_app_order_number"),
          vn: get_optional_string(row, "vn"),
        },
      );
    }
  }

  for row in &inhouse_rows {
    let hn = row.try_get::<String, _>("hn").unwrap_or_default();
    let date = match get_optional_date_string(row, "lab_date") {
      Some(d) if !d.is_empty() => d,
      _ => continue,
    };
    let result_str: String = row.try_get("result").unwrap_or_default();
    if let Ok(value) = result_str.trim().parse::<f64>() {
      grouped.entry(hn).or_default().insert(
        date.clone(),
        InrRecord {
          date,
          value,
          source: "lab_order".to_string(),
          lab_order_number: get_optional_string(row, "lab_order_number"),
          vn: get_optional_string(row, "vn"),
        },
      );
    }
  }

  Ok(
    grouped
      .into_iter()
      .map(|(hn, records)| (hn, records.into_values().collect()))
      .collect(),
  )
}

pub async fn get_dashboard_patient_data(
  config: &DbConfig,
  hns: &[String],
) -> Result<(
  HashMap<String, HosxpPatient>,
  HashMap<String, Vec<InrRecord>>,
)> {
  let pool = create_pool(config).await?;
  let patients = get_hosxp_patients_by_hns_with_pool(&pool, hns).await?;
  let inr_histories = get_inr_history_by_hns_with_pool(&pool, hns).await?;
  Ok((patients, inr_histories))
}

// ── Drug dispensing search ────────────────────────────────────────────────────

/// Queries all patients who have ever received any of the three warfarin codes.
/// Applies optional date / enrollment filters and returns server-side paginated results.
/// Searches `HOSxP` for patients who have ever received warfarin, joining
/// `opitemrece` with `patient` and `drugitems` for the three warfarin icodes,
/// grouped by `hn`.
///
/// # Errors
///
/// Propagates `sqlx::Error` if the `HOSxP` `MySQL` connection or any query
/// fails, or if a row column cannot be decoded.
// SQL `COUNT(*)` and `total_visits` results are non-negative counts that
// fit `usize` on all practical targets; the i64→usize casts are safe here.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub async fn search_hosxp_warfarin_patients(
  config: &DbConfig,
  filters: &SearchFilters,
  enrolled_hns: &[String],
) -> Result<SearchResponse> {
  let pool = create_pool(config).await?;
  let (date_from, date_to) = resolve_search_window(filters)?;
  let keyword = filters.keyword.as_deref().unwrap_or("").trim().to_string();
  let page = filters.page.max(1);
  let page_size = filters.page_size.clamp(1, 200);
  let offset = i64::from(page.saturating_sub(1) * page_size);
  let enrolled_hn_set: HashSet<&str> = enrolled_hns.iter().map(String::as_str).collect();

  if matches!(filters.enrollment_status.as_deref(), Some("enrolled")) && enrolled_hns.is_empty() {
    return Ok(SearchResponse {
      items: Vec::new(),
      total: 0,
    });
  }

  let count_select = "SELECT COUNT(DISTINCT p.hn)";
  let total_row = build_screening_query(
    count_select,
    &date_from,
    &date_to,
    &keyword,
    filters.enrollment_status.as_deref(),
    enrolled_hns,
  )
  .build()
  .fetch_one(&pool)
  .await
  .context("failed to count HOSxP warfarin patients")?;
  let total = total_row
    .try_get::<i64, _>(0)
    .context("failed to read HOSxP screening count")? as usize;

  let mut items_query = build_screening_query(
    r"
        SELECT
            p.hn,
            p.pname,
            p.fname,
            p.lname,
            p.birthday,
            p.sex,
            MIN(o.vstdate)            AS first_dispense_date,
            MAX(o.vstdate)            AS last_dispense_date,
            COUNT(DISTINCT o.vstdate) AS total_visits,
            GROUP_CONCAT(DISTINCT d.strength ORDER BY d.strength SEPARATOR ',') AS strengths
        ",
    &date_from,
    &date_to,
    &keyword,
    filters.enrollment_status.as_deref(),
    enrolled_hns,
  );
  items_query.push(
        " GROUP BY p.hn, p.pname, p.fname, p.lname, p.birthday, p.sex ORDER BY MAX(o.vstdate) DESC LIMIT ",
    );
  items_query
    .push_bind(i64::from(page_size))
    .push(" OFFSET ")
    .push_bind(offset);
  let rows = items_query
    .build()
    .fetch_all(&pool)
    .await
    .context("failed to search HOSxP warfarin patients")?;

  let mut records: Vec<PatientDrugRecord> = rows
    .iter()
    .map(|r| {
      let hn: String = r.get("hn");
      let is_enrolled = enrolled_hn_set.contains(hn.as_str());
      let strengths: String = r.try_get("strengths").unwrap_or_default();
      let strengths_vec: Vec<String> = strengths
        .split(',')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();

      PatientDrugRecord {
        hn,
        pname: r.try_get("pname").unwrap_or_default(),
        fname: r.try_get("fname").unwrap_or_default(),
        lname: r.try_get("lname").unwrap_or_default(),
        birthday: get_date_string(r, "birthday"),
        sex: r.try_get("sex").unwrap_or_default(),
        phone: r.try_get("phone").ok(),
        first_dispense_date: get_date_string(r, "first_dispense_date"),
        last_dispense_date: get_date_string(r, "last_dispense_date"),
        total_dispense_visits: r.try_get::<i64, _>("total_visits").unwrap_or(0) as usize,
        strengths_received: strengths_vec,
        is_enrolled,
        enrollment_status: None,
      }
    })
    .collect();
  records.shrink_to_fit();

  Ok(SearchResponse {
    items: records,
    total,
  })
}

pub async fn get_dispensing_history(config: &DbConfig, hn: &str) -> Result<Vec<DispensingRecord>> {
  let pool = create_pool(config).await?;
  let rows = match sqlx::query(
    r"
        SELECT
          o.hn,
          o.vn,
          o.an,
          o.vstdate,
            o.icode,
            COALESCE(d.name, 'Warfarin')   AS drug_name,
            COALESCE(d.strength, '')       AS strength,
            CAST(o.qty AS DOUBLE)          AS qty,
            CAST(o.unitprice AS DOUBLE)    AS unitprice,
            CAST(o.drugusage AS CHAR)      AS drugusage_code,
            CAST(o.sp_use AS CHAR)         AS sp_use_code,
            NULLIF(
              TRIM(
                CONCAT_WS(
                  ' ',
                  NULLIF(TRIM(CONCAT_WS(' ', du.name1, du.name2, du.name3, du.shortlist)), ''),
                  NULLIF(TRIM(CONCAT_WS(' ', su.name1, su.name2, su.name3)), '')
                )
              ),
              ''
            ) AS usage_text
        FROM opitemrece o
        LEFT JOIN drugitems d ON d.icode = o.icode
        LEFT JOIN drugusage du ON du.drugusage = o.drugusage
        LEFT JOIN sp_use su ON su.sp_use = o.sp_use
        WHERE o.hn = ?
          AND o.icode IN ('1600014', '1600013', '1600024')
        ORDER BY o.vstdate DESC, o.vn DESC, o.icode
        ",
  )
  .bind(hn)
  .fetch_all(&pool)
  .await
  {
    Ok(rows) => rows,
    Err(_) => sqlx::query(
      r"
        SELECT
          o.hn,
          o.vn,
          o.an,
          o.vstdate,
            o.icode,
            COALESCE(d.name, 'Warfarin')   AS drug_name,
            COALESCE(d.strength, '')       AS strength,
            CAST(o.qty AS DOUBLE)          AS qty,
            CAST(o.unitprice AS DOUBLE)    AS unitprice,
            NULL                           AS drugusage_code,
            NULL                           AS sp_use_code,
            NULL                           AS usage_text
        FROM opitemrece o
        LEFT JOIN drugitems d ON d.icode = o.icode
        WHERE o.hn = ?
          AND o.icode IN ('1600014', '1600013', '1600024')
        ORDER BY o.vstdate DESC, o.vn DESC, o.icode
        ",
    )
    .bind(hn)
    .fetch_all(&pool)
    .await
    .context("failed to query warfarin dispensing history")?,
  };

  Ok(
    rows
      .iter()
      .map(|r| {
        let strength: String = r.try_get("strength").unwrap_or_default();
        let usage_text = get_optional_string(r, "usage_text")
          .map(|value| value.trim().to_string())
          .filter(|value| !value.is_empty());
        let parsed_usage = usage_text
          .as_deref()
          .map(|value| parse_dispensing_usage(&strength, value));

        DispensingRecord {
          hn: r.get("hn"),
          vn: get_optional_string(r, "vn"),
          an: get_optional_string(r, "an"),
          vstdate: get_date_string(r, "vstdate"),
          icode: r.get("icode"),
          drug_name: r.try_get("drug_name").unwrap_or_default(),
          strength,
          qty: r.try_get("qty").unwrap_or(0.0),
          unitprice: r.try_get("unitprice").unwrap_or(0.0),
          drugusage_code: get_optional_string(r, "drugusage_code"),
          sp_use_code: get_optional_string(r, "sp_use_code"),
          usage_text,
          parsed_dose: parsed_usage.as_ref().and_then(|value| value.dose.clone()),
          usage_parse_note: parsed_usage.and_then(|value| value.note),
        }
      })
      .collect(),
  )
}

// ── INR history (dual source) ─────────────────────────────────────────────────

/// Fetches the complete INR history for a patient from **both** lab sources:
/// - `lab_order` (in-house) via `lab_head`
/// - `lab_app_order` (external/app) via `lab_app_head`
///
/// Results are merged, deduplicated by date (in-house preferred), and sorted
/// chronologically.
pub async fn get_inr_history(config: &DbConfig, hn: &str) -> Result<Vec<InrRecord>> {
  let pool = create_pool(config).await?;

  // In-house lab — follows the same join pattern as the TB project reference query.
  let inhouse_rows = sqlx::query(
    r"
        SELECT
            lh.lab_order_number,
            lh.vn,
            lh.order_date                AS lab_date,
            lo.lab_order_result          AS result
        FROM lab_order  lo
        JOIN lab_head   lh ON lh.lab_order_number = lo.lab_order_number
        WHERE lh.hn = ?
          AND lo.lab_items_code = '751'
          AND lo.lab_order_result IS NOT NULL
          AND lo.lab_order_result <> ''
          AND lo.lab_order_result REGEXP '^[0-9]'
        ORDER BY lh.order_date ASC
        ",
  )
  .bind(hn)
  .fetch_all(&pool)
  .await
  .context("failed to query in-house INR history")?;

  // External / app lab.
  let app_rows = sqlx::query(
    r"
        SELECT
            ah.lab_app_order_number,
            ah.vn,
            ah.order_date        AS lab_date,
            ao.lab_order_result  AS result
        FROM lab_app_order  ao
        JOIN lab_app_head   ah ON ah.lab_app_order_number = ao.lab_app_order_number
        WHERE ah.hn = ?
          AND ao.lab_items_code = '751'
          AND ao.lab_order_result IS NOT NULL
          AND ao.lab_order_result <> ''
          AND ao.lab_order_result REGEXP '^[0-9]'
        ORDER BY ah.order_date ASC
        ",
  )
  .bind(hn)
  .fetch_all(&pool)
  .await
  .context("failed to query external INR history")?;

  // Merge: in-house wins on same date.
  let mut map: std::collections::BTreeMap<String, InrRecord> = std::collections::BTreeMap::new();

  // Insert external first so in-house overwrites on same date.
  for row in &app_rows {
    // Use the robust helper that handles NaiveDate, NaiveDateTime, and String types.
    let date = match get_optional_date_string(row, "lab_date") {
      Some(d) if !d.is_empty() => d,
      _ => continue,
    };
    let result_str: String = row.try_get("result").unwrap_or_default();
    if let Ok(value) = result_str.trim().parse::<f64>() {
      map.insert(
        date.clone(),
        InrRecord {
          date,
          value,
          source: "lab_app_order".to_string(),
          lab_order_number: get_optional_string(row, "lab_app_order_number"),
          vn: get_optional_string(row, "vn"),
        },
      );
    }
  }

  for row in &inhouse_rows {
    let date = match get_optional_date_string(row, "lab_date") {
      Some(d) if !d.is_empty() => d,
      _ => continue,
    };
    let result_str: String = row.try_get("result").unwrap_or_default();
    if let Ok(value) = result_str.trim().parse::<f64>() {
      map.insert(
        date.clone(),
        InrRecord {
          date,
          value,
          source: "lab_order".to_string(),
          lab_order_number: get_optional_string(row, "lab_order_number"),
          vn: get_optional_string(row, "vn"),
        },
      );
    }
  }

  Ok(map.into_values().collect())
}

/// Returns only the most recent INR record.
pub async fn get_latest_inr(config: &DbConfig, hn: &str) -> Result<Option<InrRecord>> {
  let mut history = get_inr_history(config, hn).await?;
  Ok(history.pop())
}
