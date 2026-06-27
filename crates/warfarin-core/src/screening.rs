//! Pure helpers for the patient screening (HOSxP drug search) module.
//!
//! Input normalisation that used to live inline in the Tauri command. Kept
//! here so the clamping rules are unit-testable without a MySQL/SQLite
//! connection and reusable outside the desktop app.

use crate::models::patient::SearchFilters;

/// Hard cap on `page_size` regardless of what the caller sends. Stops a
/// malicious or buggy caller from asking the MySQL host for tens of
/// thousands of rows in a single round trip. 200 is well above the
/// practical row count a clinician can scan on a single page.
pub const MAX_PAGE_SIZE: u32 = 200;
/// Maximum number of characters retained from a search keyword. Long
/// keywords are truncated rather than rejected so the FE keeps working.
pub const MAX_KEYWORD_LEN: usize = 200;
/// Default page size applied when the caller sends `0` (or omits a value
/// that serde defaults to zero).
const DEFAULT_PAGE_SIZE: u32 = 50;
/// Default 1-based page applied when the caller sends `0`.
const DEFAULT_PAGE: u32 = 1;

/// Clamps a caller-supplied [`SearchFilters`] to safe, sensible ranges:
///
/// - `page_size` of `0` becomes `50`; values above [`MAX_PAGE_SIZE`] are
///   capped at `MAX_PAGE_SIZE`.
/// - `page` of `0` becomes `1` (1-based paging).
/// - `keyword` is trimmed; empty/whitespace-only keywords become `None`;
///   over-long keywords are truncated to [`MAX_KEYWORD_LEN`] characters.
///
/// Returns the normalised filters so the command can pass them straight to
/// the data layer.
pub fn normalize_search_filters(mut filters: SearchFilters) -> SearchFilters {
  if filters.page_size == 0 {
    filters.page_size = DEFAULT_PAGE_SIZE;
  }
  if filters.page_size > MAX_PAGE_SIZE {
    filters.page_size = MAX_PAGE_SIZE;
  }
  if filters.page == 0 {
    filters.page = DEFAULT_PAGE;
  }
  if let Some(keyword) = filters.keyword.as_deref() {
    let trimmed = keyword.trim();
    filters.keyword = if trimmed.is_empty() {
      None
    } else if trimmed.chars().count() > MAX_KEYWORD_LEN {
      Some(trimmed.chars().take(MAX_KEYWORD_LEN).collect())
    } else {
      Some(trimmed.to_string())
    };
  }
  filters
}

#[cfg(test)]
mod tests {
  use super::*;

  fn filters() -> SearchFilters {
    SearchFilters {
      keyword: None,
      date_from: None,
      date_to: None,
      enrollment_status: None,
      page: 0,
      page_size: 0,
    }
  }

  #[test]
  fn zero_page_size_defaults_to_50() {
    let f = normalize_search_filters(filters());
    assert_eq!(f.page_size, 50);
  }

  #[test]
  fn oversized_page_size_capped_to_max() {
    let mut f = filters();
    f.page_size = 500;
    assert_eq!(normalize_search_filters(f).page_size, MAX_PAGE_SIZE);
  }

  #[test]
  fn zero_page_defaults_to_one() {
    let f = normalize_search_filters(filters());
    assert_eq!(f.page, 1);
  }

  #[test]
  fn valid_page_and_size_are_preserved() {
    let mut f = filters();
    f.page = 3;
    f.page_size = 25;
    let out = normalize_search_filters(f);
    assert_eq!(out.page, 3);
    assert_eq!(out.page_size, 25);
  }

  #[test]
  fn whitespace_only_keyword_becomes_none() {
    let mut f = filters();
    f.keyword = Some("   ".to_string());
    assert!(normalize_search_filters(f).keyword.is_none());
  }

  #[test]
  fn keyword_is_trimmed() {
    let mut f = filters();
    f.keyword = Some("  1234  ".to_string());
    assert_eq!(normalize_search_filters(f).keyword.as_deref(), Some("1234"));
  }

  #[test]
  fn over_long_keyword_is_truncated_to_max_chars() {
    let mut f = filters();
    let long = "x".repeat(MAX_KEYWORD_LEN + 10);
    f.keyword = Some(long);
    let kept = normalize_search_filters(f)
      .keyword
      .expect("keyword retained");
    assert_eq!(kept.chars().count(), MAX_KEYWORD_LEN);
  }

  #[test]
  fn none_keyword_stays_none() {
    assert!(normalize_search_filters(filters()).keyword.is_none());
  }
}
