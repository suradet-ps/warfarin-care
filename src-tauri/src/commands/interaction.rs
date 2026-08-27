use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder, Row};
use std::collections::BTreeMap;
use tauri::State;
use warfarin_core::models::interaction::{DrugInteraction, DrugInteractionInput};
use warfarin_db::mysql;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HosxpDrugItem {
  pub icode: String,
  pub name: String,
  pub strength: String,
  pub units: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientDrugInteractionRecord {
  pub date: String,
  pub drug_name: String,
  pub strength: String,
  pub icode: String,
  pub interaction_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientDrugInteractionSummary {
  pub increase_count: i32,
  pub decrease_count: i32,
  pub trend: String,
}

#[tauri::command]
pub async fn get_all_drug_interactions(
  state: State<'_, warfarin_db::sqlite::AppState>,
) -> Result<Vec<DrugInteraction>, String> {
  state.require_auth().await?;
  warfarin_db::sqlite::get_all_drug_interactions(&state.pool)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_drug_interaction(
  state: State<'_, warfarin_db::sqlite::AppState>,
  input: DrugInteractionInput,
) -> Result<i64, String> {
  state.require_auth().await?;
  warfarin_db::sqlite::add_drug_interaction(&state.pool, &input)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_drug_interaction(
  state: State<'_, warfarin_db::sqlite::AppState>,
  id: i64,
) -> Result<(), String> {
  state.require_auth().await?;
  warfarin_db::sqlite::delete_drug_interaction(&state.pool, id)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_hosxp_drugs(
  keyword: String,
  state: State<'_, warfarin_db::sqlite::AppState>,
) -> Result<Vec<HosxpDrugItem>, String> {
  state.require_auth().await?;
  let config = crate::commands::settings::get_mysql_config_internal(&state.pool)
    .await?
    .ok_or_else(|| "MySQL not configured".to_string())?;
  let pool = mysql::create_pool(&config)
    .await
    .map_err(|e| e.to_string())?;

  let keyword_like = format!("%{}%", keyword.trim());

  let rows = sqlx::query(
    r"
            SELECT icode, name, strength, units
            FROM drugitems
            WHERE (name LIKE ? OR icode LIKE ?)
              AND name IS NOT NULL
              AND name <> ''
            ORDER BY name
            LIMIT 50
            ",
  )
  .bind(&keyword_like)
  .bind(&keyword_like)
  .fetch_all(&pool)
  .await
  .map_err(|e| e.to_string())?;

  Ok(
    rows
      .iter()
      .map(|r| HosxpDrugItem {
        icode: r.get("icode"),
        name: r.try_get("name").unwrap_or_default(),
        strength: r.try_get("strength").unwrap_or_default(),
        units: r.try_get("units").unwrap_or_default(),
      })
      .collect(),
  )
}

#[tauri::command]
// Interaction counts are small per-patient values that fit `i32`.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub async fn get_patient_drug_interactions(
  state: State<'_, warfarin_db::sqlite::AppState>,
  hn: String,
) -> Result<
  (
    Vec<PatientDrugInteractionRecord>,
    PatientDrugInteractionSummary,
  ),
  String,
> {
  state.require_auth().await?;
  let config = crate::commands::settings::get_mysql_config_internal(&state.pool)
    .await?
    .ok_or_else(|| "MySQL not configured".to_string())?;
  let interaction_icodes = warfarin_db::sqlite::get_drug_interaction_icodes(&state.pool)
    .await
    .map_err(|e| e.to_string())?;

  if interaction_icodes.is_empty() {
    return Ok((
      Vec::new(),
      PatientDrugInteractionSummary {
        increase_count: 0,
        decrease_count: 0,
        trend: "none".to_string(),
      },
    ));
  }

  // Get interaction types from SQLite
  let all_interactions = warfarin_db::sqlite::get_all_drug_interactions(&state.pool)
    .await
    .map_err(|e| e.to_string())?;

  let interaction_types: BTreeMap<String, String> = all_interactions
    .into_iter()
    .map(|i| (i.icode, i.interaction_type))
    .collect();

  let pool = mysql::create_pool(&config)
    .await
    .map_err(|e| e.to_string())?;

  // Calculate date 1 year ago from today (CE format for MySQL)
  let one_year_ago = Utc::now()
    .checked_sub_signed(chrono::Duration::days(365))
    .map_or_else(
      || "2024-01-01".to_string(),
      |d| d.format("%Y-%m-%d").to_string(),
    );

  // Build query with date filter - only last 1 year
  let mut query_builder = QueryBuilder::<MySql>::new(
    r"
            SELECT
                o.vstdate,
                COALESCE(d.name, 'Unknown') AS drug_name,
                COALESCE(d.strength, '') AS strength,
                o.icode
            FROM opitemrece o
            LEFT JOIN drugitems d ON d.icode = o.icode
            WHERE o.hn = ",
  );
  query_builder.push_bind(&hn);
  query_builder.push(" AND o.icode IN (");
  {
    let mut separated = query_builder.separated(", ");
    for icode in &interaction_icodes {
      separated.push_bind(icode);
    }
  }
  query_builder.push(") AND o.vstdate >= ");
  query_builder.push_bind(&one_year_ago);
  query_builder.push(" ORDER BY o.vstdate DESC");

  let rows = query_builder
    .build()
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;

  let records: Vec<PatientDrugInteractionRecord> = rows
    .iter()
    .map(|r| {
      let icode: String = r.get("icode");
      let interaction_type = interaction_types
        .get(&icode)
        .cloned()
        .unwrap_or_else(|| "none".to_string());
      let date = mysql::get_optional_date_string(r, "vstdate").unwrap_or_default();
      PatientDrugInteractionRecord {
        date,
        drug_name: r.try_get("drug_name").unwrap_or_default(),
        strength: r.try_get("strength").unwrap_or_default(),
        icode,
        interaction_type,
      }
    })
    .collect();

  // Deduplicate by icode - keep only the most recent date for each icode
  let mut unique_by_icode: BTreeMap<String, PatientDrugInteractionRecord> = BTreeMap::new();
  for record in records {
    unique_by_icode
      .entry(record.icode.clone())
      .or_insert(record);
  }
  let mut unique_records: Vec<PatientDrugInteractionRecord> =
    unique_by_icode.into_values().collect();
  // Sort by date descending (most recent first)
  unique_records.sort_by(|a, b| b.date.cmp(&a.date));

  let increase_count = unique_records
    .iter()
    .filter(|r| r.interaction_type == "increase")
    .count() as i32;
  let decrease_count = unique_records
    .iter()
    .filter(|r| r.interaction_type == "decrease")
    .count() as i32;

  let trend = if increase_count > decrease_count {
    "increase".to_string()
  } else if decrease_count > increase_count {
    "decrease".to_string()
  } else if increase_count == 0 && decrease_count == 0 {
    "none".to_string()
  } else {
    "neutral".to_string()
  };

  Ok((
    unique_records,
    PatientDrugInteractionSummary {
      increase_count,
      decrease_count,
      trend,
    },
  ))
}

/// Checks a patient's concurrent medications against known warfarin
/// interaction rules and returns matching interactions sorted by severity.
///
/// Uses the pure `check()` function from `warfarin-core`. The core never
/// touches the database - this command loads the rules from `SQLite` and
/// the patient's medications from `MySQL`, then passes both to the checker.
#[tauri::command]
pub async fn check_patient_interactions(
  state: State<'_, warfarin_db::sqlite::AppState>,
  hn: String,
) -> Result<Vec<warfarin_core::models::interaction::Interaction>, String> {
  state.require_auth().await?;

  // Load interaction rules from SQLite.
  let rules = warfarin_db::sqlite::get_all_drug_interactions(&state.pool)
    .await
    .map_err(|e| e.to_string())?;

  if rules.is_empty() {
    return Ok(Vec::new());
  }

  // Load interaction rule icodes for filtering.
  let rule_icodes: std::collections::HashSet<String> =
    rules.iter().map(|r| r.icode.clone()).collect();

  // Fetch patient's concurrent medications from MySQL (last 365 days).
  let config = crate::commands::settings::get_mysql_config_internal(&state.pool)
    .await?
    .ok_or_else(|| "MySQL not configured".to_string())?;
  let pool = mysql::create_pool(&config)
    .await
    .map_err(|e| e.to_string())?;

  let one_year_ago = Utc::now()
    .checked_sub_signed(chrono::Duration::days(365))
    .map_or_else(
      || "2024-01-01".to_string(),
      |d| d.format("%Y-%m-%d").to_string(),
    );

  let mut query_builder = QueryBuilder::<MySql>::new(
    r"
            SELECT DISTINCT o.icode,
                   COALESCE(d.name, 'Unknown') AS drug_name,
                   COALESCE(d.strength, '') AS strength
            FROM opitemrece o
            LEFT JOIN drugitems d ON d.icode = o.icode
            WHERE o.hn = ",
  );
  query_builder.push_bind(&hn);
  query_builder.push(" AND o.icode IN (");
  {
    let mut separated = query_builder.separated(", ");
    for icode in &rule_icodes {
      separated.push_bind(icode);
    }
  }
  query_builder.push(") AND o.vstdate >= ");
  query_builder.push_bind(&one_year_ago);

  let rows = query_builder
    .build()
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;

  let patient_medications: Vec<warfarin_core::models::interaction::PatientMedication> = rows
    .iter()
    .map(|r| {
      let icode: String = r.get("icode");
      warfarin_core::models::interaction::PatientMedication {
        drug_name: r.try_get("drug_name").unwrap_or_default(),
        strength: {
          let s: String = r.try_get("strength").unwrap_or_default();
          if s.is_empty() { None } else { Some(s) }
        },
        icode,
      }
    })
    .collect();

  // Run the pure interaction checker.
  Ok(warfarin_core::interaction::check(
    &patient_medications,
    &rules,
  ))
}
