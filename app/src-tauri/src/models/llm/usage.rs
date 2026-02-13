use crate::auth::commands::get_access_token_command;
use crate::constants::CLOUDFLARE_BACKEND_URL;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

/// Remaining cloud model uses for a single model
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "models.ts")]
pub struct CloudModelUsage {
    pub daily_limit: i32,
    pub requests_used: i32,
    pub remaining: i32,
}

/// Get remaining cloud model uses for the authenticated user.
/// Calls the Cloudflare worker `/v1/usage/remaining` endpoint.
#[tauri::command]
pub async fn get_remaining_cloud_uses() -> Result<HashMap<String, CloudModelUsage>, String> {
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

    let usage: HashMap<String, CloudModelUsage> = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse remaining uses: {}", e))?;

    Ok(usage)
}
