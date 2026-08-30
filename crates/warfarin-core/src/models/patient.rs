use serde::{Deserialize, Serialize};

use super::{dispensing::DispensingRecord, inr::InrRecord};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WfPatient {
  pub id: i64,
  pub hn: String,
  pub enrolled_at: String,
  pub enrolled_by: Option<String>,
  pub status: String,
  pub indication: Option<String>,
  pub target_inr_low: f64,
  pub target_inr_high: f64,
  pub notes: Option<String>,
  pub created_at: String,
  pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HosxpPatient {
  pub hn: String,
  pub pname: String,
  pub fname: String,
  pub lname: String,
  pub birthday: String,
  pub sex: String,
  pub addrpart: Option<String>,
  pub phone: Option<String>,
  /// Secondary contact number from `HOSxP` `patient.informtel`.
  pub inform_tel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientDrugRecord {
  pub hn: String,
  pub pname: String,
  pub fname: String,
  pub lname: String,
  pub birthday: String,
  pub sex: String,
  pub phone: Option<String>,
  pub first_dispense_date: String,
  pub last_dispense_date: String,
  pub total_dispense_visits: usize,
  pub strengths_received: Vec<String>,
  pub is_enrolled: bool,
  pub enrollment_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrollmentInput {
  pub hn: String,
  pub indication: String,
  pub target_inr_low: f64,
  pub target_inr_high: f64,
  pub enrolled_at: String,
  pub enrolled_by: String,
  pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientDetail {
  pub patient: WfPatient,
  pub hosxp_info: HosxpPatient,
  pub latest_inr: Option<InrRecord>,
  pub current_dose_mgday: Option<f64>,
  pub ttr6months: Option<f64>,
  pub next_appointment: Option<String>,
  pub inr_history: Vec<InrRecord>,
  pub dispensing_history: Vec<DispensingRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivePatientSummary {
  pub patient: WfPatient,
  pub hosxp_info: HosxpPatient,
  pub latest_inr: Option<InrRecord>,
  pub current_dose_mgday: Option<f64>,
  pub ttr6months: Option<f64>,
  pub next_appointment: Option<String>,
  /// Most recent clinic visit date from `wf_visits`, if any.
  pub last_visit_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchFilters {
  pub keyword: Option<String>,
  pub date_from: Option<String>,
  pub date_to: Option<String>,
  pub enrollment_status: Option<String>,
  pub page: u32,
  pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
  pub items: Vec<PatientDrugRecord>,
  pub total: usize,
}
