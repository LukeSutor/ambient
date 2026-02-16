use crate::auth::commands::get_access_token_command;
use crate::constants::CLOUDFLARE_BACKEND_URL;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

/// Response from the `/v1/usage/start-turn` endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct GenerationSession {
    pub session_token: String,
    pub max_calls: i32,
    pub expires_at: String,
    /// The credit cost that was charged for this turn.
    pub credit_cost: f64,
}

/// Full credit usage response from the backend.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "models.ts")]
pub struct CreditUsageResponse {
    /// The user's effective tier: "free", "premium", or "admin".
    pub user_tier: String,
    /// Global credit usage data.
    pub daily_credit_limit: f64,
    pub credits_used: f64,
    pub credits_remaining: f64,
    /// Per-model credit costs keyed by model key (e.g. "gemini-3-flash" → 1.0).
    pub model_costs: HashMap<String, f64>,
}

/// Get credit usage info for the authenticated user.
/// Returns the user's tier and global credit usage from the Cloudflare backend.
#[tauri::command]
pub async fn get_credit_usage() -> Result<CreditUsageResponse, String> {
    let access_token = get_access_token_command()
        .await?
        .ok_or_else(|| "No access token found. Please sign in.".to_string())?;

    let client = reqwest::Client::new();
    let endpoint = format!("{}/v1/usage/remaining", CLOUDFLARE_BACKEND_URL);

    let resp = client
        .get(&endpoint)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch credit usage: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Failed to fetch credit usage ({}): {}", status, text));
    }

    let response: CreditUsageResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse credit usage: {}", e))?;

    Ok(response)
}

/// Create a generation session by calling `/v1/usage/start-turn`.
///
/// This checks the rate limit and increments usage once for the entire
/// turn. Returns a session token that can be included in subsequent
/// LLM calls to bypass per-call rate limiting.
///
/// Returns `Err` with a message containing "rate_limit_exceeded" if the
/// daily limit has been reached, or "model_not_available" if the model
/// is not available on the user's tier.
pub async fn create_generation_session(model_type: &str) -> Result<GenerationSession, String> {
    let access_token = get_access_token_command()
        .await?
        .ok_or_else(|| "No access token found. Please sign in.".to_string())?;

    let client = reqwest::Client::new();
    let endpoint = format!("{}/v1/usage/start-turn", CLOUDFLARE_BACKEND_URL);

    let resp = client
        .post(&endpoint)
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&serde_json::json!({ "modelType": model_type }))
        .send()
        .await
        .map_err(|e| format!("Failed to start generation turn: {}", e))?;

    if resp.status().as_u16() == 429 {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        let msg = body.get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Daily usage limit reached");
        return Err(format!("rate_limit_exceeded: {}", msg));
    }

    if resp.status().as_u16() == 403 {
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        let error = body.get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("model_not_available");
        let msg = body.get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Model not available on your plan");
        return Err(format!("{}: {}", error, msg));
    }

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Failed to start generation turn ({}): {}", status, text));
    }

    let session: GenerationSession = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse generation session: {}", e))?;

    log::info!(
        "[usage] Created generation session for model '{}', token: {}, max_calls: {}",
        model_type, session.session_token, session.max_calls
    );

    Ok(session)
}
