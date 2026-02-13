use crate::auth::commands::get_access_token_command;
use crate::constants::CLOUDFLARE_BACKEND_URL;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

/// Usage info for a single cloud model.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "models.ts")]
pub struct CloudModelUsage {
    /// Daily request limit. -1 means unlimited.
    pub daily_limit: i32,
    pub requests_used: i32,
    /// Remaining uses today. -1 means unlimited.
    pub remaining: i32,
    /// Whether this model is accessible on the user's current tier.
    pub is_available: bool,
}

/// Full model access response from the backend.
/// Includes the user's effective tier and per-model usage data.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "models.ts")]
pub struct ModelAccessResponse {
    /// The user's effective tier: "free", "premium", or "admin".
    pub user_tier: String,
    /// Per-model usage keyed by model key (e.g. "gemini-3-flash").
    pub models: HashMap<String, CloudModelUsage>,
}

/// Get model access info for the authenticated user.
/// Returns the user's tier and per-model usage from the Cloudflare backend.
#[tauri::command]
pub async fn get_remaining_cloud_uses() -> Result<ModelAccessResponse, String> {
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
        .map_err(|e| format!("Failed to fetch remaining uses: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Failed to fetch remaining uses ({}): {}", status, text));
    }

    let response: ModelAccessResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse remaining uses: {}", e))?;

    Ok(response)
}
