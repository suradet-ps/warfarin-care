//! Pill decomposition and dispense-period summary helpers.
//!
//! Pure functions that turn a per-day dose schedule into a count of whole
//! and half pills of each strength (2/3/5 mg), plus a Thai-language
//! dispense summary used on the physician communication slip. Extracted
//! from the SQLite layer so the algorithm is unit-testable without a DB.

use std::collections::HashMap;

use chrono::{Datelike, Duration, NaiveDate};

use crate::models::visit::{
  DoseSchedule, PillLineSummary, RegimenOptionSnapshot, TotalPillsSummary,
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
pub(crate) fn pills_for_dose(dose_mg: f64) -> Vec<(u8, bool)> {
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
              let total_half_units = 2 * (5 * w5 + 3 * w3 + 2 * w2) + (5 * h5 + 3 * h3 + 2 * h2);
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
pub fn tally_pills(day_doses: &[f64]) -> HashMap<u8, (u32, u32)> {
  let mut pill_counts: HashMap<u8, (u32, u32)> = HashMap::new();
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

pub fn calculate_pills_summary(
  visit_date: &str,
  new_dose_detail: &DoseSchedule,
  next_appointment: &str,
) -> Option<TotalPillsSummary> {
  let visit = NaiveDate::parse_from_str(visit_date, "%Y-%m-%d").ok()?;
  let next = NaiveDate::parse_from_str(next_appointment, "%Y-%m-%d").ok()?;

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
    let current = visit + Duration::days(d);
    let day_index = current.weekday().num_days_from_monday() as usize;
    day_doses.push(weekly[day_index]);
  }
  let pill_counts = tally_pills(&day_doses);

  let lines: Vec<PillLineSummary> = pill_counts
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
      PillLineSummary {
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

  Some(TotalPillsSummary {
    header,
    pill_lines: lines,
  })
}

pub fn selected_option_summary(snapshot: &RegimenOptionSnapshot) -> TotalPillsSummary {
  TotalPillsSummary {
    header: snapshot.total_pills_summary.header.clone(),
    pill_lines: snapshot
      .total_pills_summary
      .pill_lines
      .iter()
      .map(|line| PillLineSummary {
        mg: line.mg,
        dispensed_count: line.dispensed_count,
        usage_note: line.usage_note.clone(),
      })
      .collect(),
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
    assert_eq!(
      total_half_units, 9,
      "4.5 mg dose should dispense 4.5 mg total"
    );
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
