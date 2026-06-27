use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrugInteraction {
  pub id: i64,
  pub icode: String,
  pub drug_name: String,
  pub strength: Option<String>,
  pub interaction_type: String,
  pub created_at: String,
  pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrugInteractionInput {
  pub icode: String,
  pub drug_name: String,
  pub strength: Option<String>,
  pub interaction_type: String,
}

pub type InteractionType = &'static str;
pub const INTERACTION_INCREASE: InteractionType = "increase";
pub const INTERACTION_DECREASE: InteractionType = "decrease";

pub const INTERACTION_TYPES: [InteractionType; 2] = [INTERACTION_INCREASE, INTERACTION_DECREASE];
