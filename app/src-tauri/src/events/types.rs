use crate::os_utils::windows::window::ApplicationTextData;
use crate::memory::types::MemoryEntry;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const CAPTURE_SCREEN: &str = "capture_screen";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct CaptureScreenEvent {
  pub timestamp: String,
}

pub const GET_SCREEN_DIFF: &str = "get_screen_diff";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct GetScreenDiffEvent {
  pub data: Vec<ApplicationTextData>,
  pub active_url: Option<String>,
  pub timestamp: String,
}

pub const DETECT_TASKS: &str = "detect_tasks";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct DetectTasksEvent {
  pub text: String,
  pub active_url: Option<String>,
  pub timestamp: String,
}

pub const SUMMARIZE_SCREEN: &str = "summarize_screen";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct SummarizeScreenEvent {
  pub text: String,
  pub data: Vec<ApplicationTextData>,
  pub active_url: Option<String>,
  pub timestamp: String,
}

pub const CHAT_STREAM: &str = "chat_stream";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct ChatStreamEvent {
  pub delta: String,
  pub is_finished: bool,
  pub full_response: String,
  pub conv_id: Option<String>,
}

pub const HUD_CHAT: &str = "hud_chat";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct HudChatEvent {
  pub text: String,
  pub ocr_responses: Vec<OcrResponseEvent>,
  pub timestamp: String,
  pub conv_id: Option<String>,
}

pub const OCR_RESPONSE: &str = "ocr_response";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct OcrResponseEvent {
  pub text: String,
  pub success: bool,
  pub timestamp: String,
}

pub const UPDATE_TASKS: &str = "update_tasks";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct UpdateTasksEvent {
  pub timestamp: String,
}

pub const EXTRACT_INTERACTIVE_MEMORY: &str = "extract_interactive_memory";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct ExtractInteractiveMemoryEvent {
  pub message: String,
  pub message_id: String,
  pub timestamp: String,
}

pub const MEMORY_EXTRACTED: &str = "memory_extracted";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct MemoryExtractedEvent {
  pub memory: MemoryEntry,
  pub timestamp: String,
}