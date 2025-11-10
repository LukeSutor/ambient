use serde::{Deserialize, Serialize};

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
