use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoseSchedule {
  pub mon: f64,
  pub tue: f64,
  pub wed: f64,
  pub thu: f64,
  pub fri: f64,
  pub sat: f64,
  pub sun: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PillLineSummary {
  pub mg: u8,
  pub dispensed_count: u32,
  pub usage_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalPillsSummary {
  pub header: String,
  pub pill_lines: Vec<PillLineSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RegimenPillRenderData {
  pub mg: u8,
  pub count: u32,
  pub is_half: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RegimenDaySchedule {
  pub day_index: u8,
  pub total_dose: f64,
  pub pills: Vec<RegimenPillRenderData>,
  pub is_stop_day: bool,
  pub is_special_day: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RegimenPillLineSummary {
  pub mg: u8,
  pub dispensed_count: u32,
  pub usage_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RegimenTotalPillsSummary {
  pub header: String,
  pub pill_lines: Vec<RegimenPillLineSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RegimenOptionSnapshot {
  pub description: String,
  pub weekly_dose_actual: f64,
  pub weekly_schedule: Vec<RegimenDaySchedule>,
  pub total_pills_summary: RegimenTotalPillsSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WfVisit {
  pub id: i64,
  pub hn: String,
  pub visit_date: String,
  pub inr_value: Option<f64>,
  pub inr_source: Option<String>,
  pub current_dose_mgday: Option<f64>,
  pub dose_detail: Option<DoseSchedule>,
  pub new_dose_mgday: Option<f64>,
  pub new_dose_detail: Option<DoseSchedule>,
  pub new_dose_description: Option<String>,
  pub dose_changed: bool,
  pub next_appointment: Option<String>,
  pub next_inr_due: Option<String>,
  pub physician: Option<String>,
  pub notes: Option<String>,
  pub side_effects: Option<Vec<String>>,
  pub adherence: Option<String>,
  pub created_by: Option<String>,
  pub created_at: String,
  pub total_pills_summary: Option<TotalPillsSummary>,
  pub selected_dose_option: Option<RegimenOptionSnapshot>,
  pub reviewed_at: Option<String>,
  pub reviewed_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisitInput {
  pub hn: String,
  pub visit_date: String,
  pub inr_value: Option<f64>,
  pub inr_source: Option<String>,
  pub current_dose_mgday: Option<f64>,
  pub dose_detail: Option<DoseSchedule>,
  pub new_dose_mgday: Option<f64>,
  pub new_dose_detail: Option<DoseSchedule>,
  pub new_dose_description: Option<String>,
  pub dose_changed: bool,
  pub next_appointment: Option<String>,
  pub next_inr_due: Option<String>,
  pub physician: Option<String>,
  pub notes: Option<String>,
  pub side_effects: Option<Vec<String>>,
  pub adherence: Option<String>,
  pub created_by: Option<String>,
  pub selected_dose_option: Option<RegimenOptionSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoseSuggestion {
  pub suggested_dose_mgweek: f64,
  pub adjustment_percent: f64,
  pub recommendation: String,
  pub urgency: String,
  pub recheck_days: u32,
}
