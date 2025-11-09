use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "memory.ts")]
pub struct MemoryEntry {
  pub id: String,
  pub message_id: String,
  pub memory_type: String,
  pub text: String,
  pub embedding: Vec<f32>,
  pub timestamp: String,
  pub similarity: Option<f64>,
}
