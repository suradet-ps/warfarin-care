//! Slip export commands - persist generated PDF output for the printable slip view.

use anyhow::{Context, Result};
use std::{
  ffi::OsStr,
  path::{Path, PathBuf},
};
use tauri::State;

use warfarin_db::sqlite::AppState;

fn validate_output_path(path: &str) -> Result<PathBuf> {
  let requested = Path::new(path);
  let file_name = requested
    .file_name()
    .context("pdf output path must include a file name")?;
  let parent = requested
    .parent()
    .filter(|value| !value.as_os_str().is_empty())
    .context("pdf output path must include a parent directory")?;
  let canonical_parent = parent
    .canonicalize()
    .with_context(|| format!("failed to resolve output directory at {}", parent.display()))?;

  let mut output_path = canonical_parent.join(file_name);
  let is_pdf = output_path
    .extension()
    .and_then(OsStr::to_str)
    .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"));

  if !is_pdf {
    output_path.set_extension("pdf");
  }

  Ok(output_path)
}

/// Saves generated slip PDF bytes to a user-selected filesystem path.
#[tauri::command]
pub async fn save_slip_pdf(
  path: String,
  bytes: Vec<u8>,
  state: State<'_, AppState>,
) -> Result<(), String> {
  state.require_auth().await?;
  if bytes.is_empty() {
    return Err("pdf content is empty".to_string());
  }

  let output_path = validate_output_path(&path).map_err(|error| error.to_string())?;

  tokio::fs::write(&output_path, bytes)
    .await
    .with_context(|| format!("failed to write PDF to {}", output_path.display()))
    .map_err(|error| error.to_string())
}
