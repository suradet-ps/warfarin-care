//! Dose suggestion and TTR calculation for warfarin clinic.
//!
//! All functions are pure with no I/O — fully unit-testable.

use crate::models::visit::DoseSuggestion;

/// Rounds a dose value to the nearest 0.5 mg/day practical step.
fn round_to_half_mg(value: f64) -> f64 {
  (value * 2.0).round() / 2.0
}

/// Computes a warfarin dose adjustment suggestion given the current dose,
/// current INR, and the patient's target INR range.
///
/// # Algorithm (per Thai clinical practice guidelines)
///
/// The decision tree uses **deltas from the patient's target range**, not
/// hard-coded population thresholds. This is critical: a patient with a
/// mechanical mitral valve (target 2.5–3.5) and INR = 3.2 must be
/// considered "in range", not "above 3.0" as it would be with absolute
/// thresholds.
///
/// | Relation to target            | Adjustment | Urgency | Recheck |
/// |-------------------------------|------------|---------|---------|
/// | Within `[target_low, target_high]` | 0%        | normal  | 28–42 d |
/// | > 1.0 above `target_high`     | -20%       | urgent  | 3 d     |
/// | 0.5–1.0 above `target_high`   | -15%       | caution | 7 d     |
/// | < 0.5 above `target_high`     | -10%       | caution | 14 d    |
/// | 0.5–1.0 above `target_high` AND INR ≥ 5.0 | hold 1–2d + Vit K, -20% | hold | 3–5 d |
/// | INR ≥ 5.0 with `target_high < 4.0` | hold + Vit K | hold | 3–5 d |
/// | 0.5–1.0 below `target_low`    | +10%       | caution | 14 d    |
/// | > 1.0 below `target_low`      | +20%       | urgent  | 7–14 d  |
/// | INR < 1.5 (any target)        | +20%       | urgent  | 7–14 d  |
pub fn suggest_dose(
  current_dose_weekly: f64,
  inr: f64,
  target_low: f64,
  target_high: f64,
) -> DoseSuggestion {
  // Guard against invalid ranges. Fall back to the most common 2.0–3.0.
  // Treats any inversion or non-finite value as "use defaults" rather than
  // trying to repair one side and leaving the other unrepaired.
  let (target_low, target_high) = if target_low.is_finite()
    && target_high.is_finite()
    && target_low > 0.0
    && target_high > target_low
  {
    (target_low, target_high)
  } else {
    (2.0_f64, 3.0_f64)
  };

  // Universal critical-low threshold (medical emergency, regardless of target).
  // Per AGENTS.md §Key Business Rules #9: "INR > 5.0: Always surface as a
  // critical alert regardless of target range".
  let universal_critical_high = 5.0;
  let universal_critical_low = 1.5;

  let in_range = inr >= target_low && inr <= target_high;
  let above = inr - target_high;
  let below = target_low - inr;

  // Above-range branches, ordered most-severe first.
  let (adjustment_percent, recommendation, urgency, recheck_days): (f64, &str, &str, u32) =
    if inr >= universal_critical_high {
      // Hold + Vitamin K per critical threshold.
      (
        -20.0,
        "หยุดยา 1-2 วัน และให้ Vitamin K 1-2 mg PO และลดขนาดยา 20%",
        "hold",
        3,
      )
    } else if inr >= 4.0 && target_high < 4.0 {
      // Hold for elevated INR even when not at universal critical, but only
      // when patient's target is below 4.0 (otherwise 4.0 is in range).
      (-15.0, "หยุดยา 1 วัน และลดขนาดยา 15%", "hold", 5)
    } else if above > 1.0 {
      (-20.0, "ลดขนาดยา 15-20% นัดตรวจ INR ใหม่ใน 7 วัน", "urgent", 7)
    } else if above > 0.5 {
      (
        -15.0,
        "ลดขนาดยา 10-15% นัดตรวจ INR ใหม่ใน 7-14 วัน",
        "caution",
        10,
      )
    } else if above > 0.0 {
      (
        -10.0,
        "ลดขนาดยา 5-10% นัดตรวจ INR ใหม่ใน 14 วัน",
        "caution",
        14,
      )
    } else if in_range {
      (0.0, "คงขนาดยาเดิม นัดตรวจ INR ใน 4-6 สัปดาห์", "normal", 35)
    } else if inr < universal_critical_low {
      // Below 1.5 is universally critical regardless of target.
      (
        20.0,
        "เพิ่มขนาดยา 15-20% นัดตรวจ INR ใหม่ใน 7-14 วัน (เสี่ยงลิ่มเลือด)",
        "urgent",
        10,
      )
    } else if below > 1.0 {
      (
        20.0,
        "เพิ่มขนาดยา 15-20% นัดตรวจ INR ใหม่ใน 7-14 วัน",
        "urgent",
        10,
      )
    } else if below > 0.5 {
      (
        15.0,
        "เพิ่มขนาดยา 10-15% นัดตรวจ INR ใหม่ใน 14 วัน",
        "caution",
        14,
      )
    } else {
      // below > 0.0 (just below target_low)
      (
        10.0,
        "เพิ่มขนาดยา 5-10% นัดตรวจ INR ใหม่ใน 14 วัน",
        "caution",
        14,
      )
    };

  let suggested_dose_weekly =
    round_to_half_mg(current_dose_weekly * (1.0 + adjustment_percent / 100.0)).max(0.0);

  DoseSuggestion {
    suggested_dose_mgweek: suggested_dose_weekly,
    adjustment_percent,
    recommendation: recommendation.to_string(),
    urgency: urgency.to_string(),
    recheck_days,
  }
}

/// Calculates Time in Therapeutic Range (TTR) using the Rosendaal linear
/// interpolation method.
///
/// # Arguments
/// * `inr_records` — slice of `(date_str, inr_value)` pairs, any order.
/// * `target_low` — lower bound of therapeutic range.
/// * `target_high` — upper bound of therapeutic range.
/// * `window_days` — only consider INR readings within this many days from today.
///   Pass `u32::MAX` to use all available data.
///
/// # Returns
/// TTR as a percentage (0.0 – 100.0), or `None` if there are fewer than 2
/// readings in the window.
pub fn calculate_ttr(
  inr_records: &[(String, f64)],
  target_low: f64,
  target_high: f64,
  window_days: u32,
) -> Option<f64> {
  // Parse and sort records chronologically.
  let mut records: Vec<(chrono::NaiveDate, f64)> = inr_records
    .iter()
    .filter_map(|(date_str, value)| {
      chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .ok()
        .map(|d| (d, *value))
    })
    .collect();
  records.sort_by_key(|(d, _)| *d);

  // Apply window filter.
  if window_days < u32::MAX {
    let cutoff = chrono::Utc::now().date_naive() - chrono::Duration::days(i64::from(window_days));
    records.retain(|(d, _)| *d >= cutoff);
  }

  if records.len() < 2 {
    return None;
  }

  let mut total_days: i64 = 0;
  let mut in_range_days: i64 = 0;

  for window in records.windows(2) {
    let (d1, inr1) = window[0];
    let (d2, inr2) = window[1];

    let span = (d2 - d1).num_days();
    if span <= 0 {
      continue;
    }

    // For each day in the interval, linearly interpolate the INR.
    for day_offset in 0..span {
      let fraction = day_offset as f64 / span as f64;
      let interpolated_inr = inr1 + fraction * (inr2 - inr1);

      total_days += 1;
      if interpolated_inr >= target_low && interpolated_inr <= target_high {
        in_range_days += 1;
      }
    }
  }

  if total_days == 0 {
    return None;
  }

  Some((in_range_days as f64 / total_days as f64) * 100.0)
}

#[cfg(test)]
mod tests {
  use super::*;

  // ── suggest_dose tests ──────────────────────────────────────────────────

  #[test]
  fn suggest_dose_in_range_returns_no_change() {
    // Patient target 2.0-3.0, INR 2.5 — in range.
    let result = suggest_dose(35.0, 2.5, 2.0, 3.0);
    assert_eq!(result.adjustment_percent, 0.0);
    assert_eq!(result.urgency, "normal");
    assert_eq!(result.suggested_dose_mgweek, 35.0);
    assert_eq!(result.recheck_days, 35);
  }

  #[test]
  fn suggest_dose_just_above_target_decreases_10_percent() {
    // Patient target 2.0-3.0, INR 3.3 — 0.3 above high → -10%.
    let result = suggest_dose(35.0, 3.3, 2.0, 3.0);
    assert_eq!(result.adjustment_percent, -10.0);
    assert_eq!(result.urgency, "caution");
    // 35.0 * 0.90 = 31.5
    assert_eq!(result.suggested_dose_mgweek, 31.5);
  }

  #[test]
  fn suggest_dose_above_target_0_5_to_1_0_decreases_15_percent() {
    // INR 3.7 = 0.7 above target_high 3.0 → above > 0.5 && <= 1.0 → -15%.
    let result = suggest_dose(35.0, 3.7, 2.0, 3.0);
    assert_eq!(result.adjustment_percent, -15.0);
    assert_eq!(result.urgency, "caution");
    // 35.0 * 0.85 = 29.75 → rounded to 30.0
    assert_eq!(result.suggested_dose_mgweek, 30.0);
  }

  #[test]
  fn suggest_dose_4_to_5_hold_and_reduce_15() {
    // INR 4.5, target 2.0-3.0 → 4.0+ but target_high < 4.0 → -15%, hold.
    let result = suggest_dose(35.0, 4.5, 2.0, 3.0);
    assert_eq!(result.adjustment_percent, -15.0);
    assert_eq!(result.urgency, "hold");
    // 35.0 * 0.85 = 29.75 → 30.0
    assert_eq!(result.suggested_dose_mgweek, 30.0);
    assert_eq!(result.recheck_days, 5);
  }

  #[test]
  fn suggest_dose_over_5_hold_vit_k() {
    // INR 5.5, target 2.0-3.0 → universal_critical_high → -20%, hold.
    let result = suggest_dose(35.0, 5.5, 2.0, 3.0);
    assert_eq!(result.adjustment_percent, -20.0);
    assert_eq!(result.urgency, "hold");
    // 35.0 * 0.80 = 28.0
    assert_eq!(result.suggested_dose_mgweek, 28.0);
    assert_eq!(result.recheck_days, 3);
  }

  #[test]
  fn suggest_dose_just_below_target_increases_10_percent() {
    // INR 1.8, target 2.0-3.0 → 0.2 below target_low → +10%.
    let result = suggest_dose(35.0, 1.8, 2.0, 3.0);
    assert_eq!(result.adjustment_percent, 10.0);
    assert_eq!(result.urgency, "caution");
    // 35.0 * 1.10 = 38.5
    assert_eq!(result.suggested_dose_mgweek, 38.5);
  }

  #[test]
  fn suggest_dose_below_1_5_increases_20_percent() {
    // INR 1.3, any target → universal_critical_low → +20%, urgent.
    let result = suggest_dose(35.0, 1.3, 2.0, 3.0);
    assert_eq!(result.adjustment_percent, 20.0);
    assert_eq!(result.urgency, "urgent");
    // 35.0 * 1.20 = 42.0
    assert_eq!(result.suggested_dose_mgweek, 42.0);
  }

  #[test]
  fn suggest_dose_rounds_to_half_mg() {
    // 28.0 * 1.10 = 30.8 → rounds to 31.0
    let result = suggest_dose(28.0, 1.8, 2.0, 3.0);
    assert_eq!(result.suggested_dose_mgweek, 31.0);
  }

  #[test]
  fn suggest_dose_zero_dose_stays_zero() {
    let result = suggest_dose(0.0, 1.0, 2.0, 3.0);
    assert_eq!(result.suggested_dose_mgweek, 0.0);
  }

  // ── Target-range aware tests (R-1.1) ──────────────────────────────────

  #[test]
  fn suggest_dose_mech_mitral_target_2_5_3_5_inr_3_2_in_range() {
    // Mechanical mitral valve: target 2.5-3.5. INR 3.2 is in range.
    let result = suggest_dose(35.0, 3.2, 2.5, 3.5);
    assert_eq!(result.adjustment_percent, 0.0);
    assert_eq!(result.urgency, "normal");
    assert_eq!(result.suggested_dose_mgweek, 35.0);
  }

  #[test]
  fn suggest_dose_mech_mitral_target_2_5_3_5_inr_2_4_below() {
    // INR 2.4, target 2.5-3.5 → 0.1 below → +10%.
    let result = suggest_dose(35.0, 2.4, 2.5, 3.5);
    assert_eq!(result.adjustment_percent, 10.0);
    assert_eq!(result.urgency, "caution");
  }

  #[test]
  fn suggest_dose_mech_mitral_target_2_5_3_5_inr_3_7_above_0_5() {
    // INR 3.7, target 2.5-3.5 → 0.2 above → -10%.
    let result = suggest_dose(35.0, 3.7, 2.5, 3.5);
    assert_eq!(result.adjustment_percent, -10.0);
    assert_eq!(result.urgency, "caution");
  }

  #[test]
  fn suggest_dose_mech_mitral_target_2_5_3_5_inr_4_5_hold() {
    // INR 4.5, target 2.5-3.5 → target_high (3.5) < 4.0 → -15%, hold.
    let result = suggest_dose(35.0, 4.5, 2.5, 3.5);
    assert_eq!(result.adjustment_percent, -15.0);
    assert_eq!(result.urgency, "hold");
  }

  #[test]
  fn suggest_dose_mech_aortic_target_2_0_3_0_inr_3_2_caution() {
    // Bileaflet aortic: same as default 2.0-3.0. INR 3.2 = 0.2 above → -10%.
    let result = suggest_dose(35.0, 3.2, 2.0, 3.0);
    assert_eq!(result.adjustment_percent, -10.0);
    assert_eq!(result.urgency, "caution");
  }

  #[test]
  fn suggest_dose_inr_5_0_always_critical() {
    // Even with target 2.5-3.5, INR 5.0 is universal critical.
    let result = suggest_dose(35.0, 5.0, 2.5, 3.5);
    assert_eq!(result.urgency, "hold");
    assert_eq!(result.adjustment_percent, -20.0);
  }

  #[test]
  fn suggest_dose_inr_1_4_always_critical() {
    // INR 1.4 is below 1.5 → universal critical low → +20%, urgent.
    let result = suggest_dose(35.0, 1.4, 2.5, 3.5);
    assert_eq!(result.urgency, "urgent");
    assert_eq!(result.adjustment_percent, 20.0);
  }

  #[test]
  fn suggest_dose_invalid_target_falls_back_to_default() {
    // target_low=0, target_high=0 → fallback to 2.0-3.0.
    let result = suggest_dose(35.0, 2.5, 0.0, 0.0);
    assert_eq!(result.adjustment_percent, 0.0);
    assert_eq!(result.urgency, "normal");
  }

  #[test]
  fn suggest_dose_target_low_greater_than_high_falls_back() {
    // target_low > target_high → fallback to 2.0-3.0.
    let result = suggest_dose(35.0, 2.5, 4.0, 3.0);
    assert_eq!(result.adjustment_percent, 0.0);
    assert_eq!(result.urgency, "normal");
  }

  #[test]
  fn suggest_dose_boundary_inr_equals_target_low_is_in_range() {
    let result = suggest_dose(35.0, 2.0, 2.0, 3.0);
    assert_eq!(result.adjustment_percent, 0.0);
    assert_eq!(result.urgency, "normal");
  }

  #[test]
  fn suggest_dose_boundary_inr_equals_target_high_is_in_range() {
    let result = suggest_dose(35.0, 3.0, 2.0, 3.0);
    assert_eq!(result.adjustment_percent, 0.0);
    assert_eq!(result.urgency, "normal");
  }

  // ── calculate_ttr tests ─────────────────────────────────────────────────

  #[test]
  fn ttr_all_in_range_returns_100() {
    let records = vec![
      ("2024-01-01".to_string(), 2.5),
      ("2024-01-08".to_string(), 2.5),
      ("2024-01-15".to_string(), 2.5),
    ];
    let ttr = calculate_ttr(&records, 2.0, 3.0, u32::MAX).unwrap();
    assert!((ttr - 100.0).abs() < 0.01, "expected 100%, got {ttr:.2}%");
  }

  #[test]
  fn ttr_all_out_of_range_returns_0() {
    let records = vec![
      ("2024-01-01".to_string(), 4.0),
      ("2024-01-08".to_string(), 4.0),
    ];
    let ttr = calculate_ttr(&records, 2.0, 3.0, u32::MAX).unwrap();
    assert!((ttr - 0.0).abs() < 0.01, "expected 0%, got {ttr:.2}%");
  }

  #[test]
  fn ttr_half_in_range_returns_approximately_50() {
    // Days 1–7: INR interpolates 2.0→4.0, passes through 3.0 at midpoint.
    // Days in range: those where interpolated INR <= 3.0.
    let records = vec![
      ("2024-01-01".to_string(), 2.0),
      ("2024-01-15".to_string(), 4.0),
    ];
    let ttr = calculate_ttr(&records, 2.0, 3.0, u32::MAX).unwrap();
    // Should be approximately 50% (first half of the period is in range)
    assert!(ttr > 40.0 && ttr < 60.0, "expected ~50%, got {ttr:.2}%");
  }

  #[test]
  fn ttr_fewer_than_2_records_returns_none() {
    let records = vec![("2024-01-01".to_string(), 2.5)];
    assert!(calculate_ttr(&records, 2.0, 3.0, u32::MAX).is_none());
  }

  #[test]
  fn ttr_empty_returns_none() {
    assert!(calculate_ttr(&[], 2.0, 3.0, u32::MAX).is_none());
  }
}
