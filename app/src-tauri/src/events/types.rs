use crate::os_utils::windows::window::ApplicationTextData;
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

pub const GET_USER_ACK: &str = "get_user_ack";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct GetUserAckEvent {
  pub message: String,
  pub timestamp: String,
}

pub const USER_ACK: &str = "user_ack";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct UserAckEvent {
  pub acknowledged: bool,
  pub timestamp: String,
}

pub const GET_USER_INPUT: &str = "get_user_input";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct GetUserInputEvent {
  pub message: String,
  pub timestamp: String,
}

pub const USER_INPUT: &str = "user_input";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct UserInputEvent {
  pub message: String,
  pub timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct FunctionArgument {
  pub key: String,
  pub value: String,
}

pub const EXECUTE_FUNCTION: &str = "execute_function";
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct ExecuteFunctionEvent {
  pub function_name: String,
  pub args: Vec<FunctionArgument>,
  pub timestamp: String,
}
