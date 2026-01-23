use crate::memory::types::MemoryEntry;
use crate::db::conversations::{Attachment, Message};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

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
pub struct AttachmentData {
  pub name: String,
  pub file_type: String,
  pub data: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct HudChatEvent {
  pub text: String,
  pub timestamp: String,
  pub conv_id: String,
  pub message_id: String,
  pub attachments: Vec<AttachmentData>,
}

pub const ATTACHMENTS_CREATED: &str = "attachments_created";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct AttachmentsCreatedEvent {
  pub message_id: String,
  pub attachments: Vec<Attachment>,
  pub timestamp: String,
}

pub const OCR_RESPONSE: &str = "ocr_response";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct OcrResponseEvent {
  pub text: String,
  pub success: bool,
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

pub const GENERATE_CONVERSATION_NAME: &str = "generate_conversation_name";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct GenerateConversationNameEvent {
  pub conv_id: String,
  pub message: String,
  pub timestamp: String,
}

pub const RENAME_CONVERSATION: &str = "rename_conversation";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct RenameConversationEvent {
  pub conv_id: String,
  pub new_name: String,
  pub timestamp: String,
}

pub const COMPUTER_USE_UPDATE: &str = "computer_use_update";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct ComputerUseUpdateEvent {
  pub status: String,
  pub message: Message,
}

pub const COMPUTER_USE_TOAST: &str = "computer_use_toast";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct ComputerUseToastEvent {
  pub message: String,
  pub timestamp: String,
}

pub const GET_SAFETY_CONFIRMATION: &str = "get_safety_confirmation";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct SafetyConfirmationEvent {
  pub reason: String,
  pub timestamp: String,
}

pub const SAFETY_CONFIRMATION_RESPONSE: &str = "safety_confirmation_response";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct SafetyConfirmationResponseEvent {
  pub user_confirmed: bool,
  pub timestamp: String,
}
