use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ActionResponse {
    pub function_name: String,
    pub args: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum ComputerAction {
    OpenWebBrowser,
    Wait5Seconds,
    GoBack,
    GoForward,
    Search,
    Navigate { url: String },
    ClickAt { x: i32, y: i32 },
    HoverAt { x: i32, y: i32 },
    TypeTextAt { x: i32, y: i32, text: String, press_enter: Option<bool>, clear_before_typing: Option<bool> },
    KeyCombination { keys: String },
    ScrollDocument { direction: String },
    ScrollAt { x: i32, y: i32, direction: String, magnitude: Option<i32> },
    DragAndDrop { x: i32, y: i32, destination_x: i32, destination_y: i32 },
}

/// Response types for Gemini API
#[derive(Debug, Deserialize, Serialize)]
pub struct GeminiResponse {
    pub candidates: Option<Vec<Candidate>>,
    pub error: Option<GeminiError>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Candidate {
    pub content: Content,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Content {
    pub parts: Vec<Part>,
    pub role: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Part {
    Text { text: String },
    FunctionCall { function_call: FunctionCall },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeminiError {
    pub code: i32,
    pub message: String,
    pub status: String,
}
