use reqwest;
use std::process::{Command, Stdio};


#[tauri::command]
pub fn start_server() -> Result<String, String> {
    println!("[tauri] Starting server...");
    // Spawn the command
    let _child = Command::new("C:\\Users\\Luke\\Desktop\\coding\\local-computer-use\\src-tauri\\binaries\\qwen2vl-server-x86_64-pc-windows-msvc.exe")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    println!("[tauri] Server started.");
    Ok("Command is running.".to_string())
}

#[tauri::command]
pub async fn shutdown_server() -> Result<String, String> {
    println!("[tauri] Shutting down server...");
    let client = reqwest::Client::new();
    match client.post("http://localhost:8008/shutdown")
        .send()
        .await {
            Ok(res) => {
                if res.status().is_success() {
                    println!("[tauri] Server shut down.");
                    Ok("Server shutdown request sent successfully.".to_string())
                } else {
                    println!("[tauri] Server failed to shut down.");
                    Err(format!("Failed to shutdown server: {}", res.status()))
                }
            },
            Err(e) => Err(format!("Failed to send request: {}", e)),
        }
}

#[tauri::command]
pub async fn infer(prompt: String, image: String) -> Result<String, String> {
    let client = reqwest::Client::new();
    let request_body = serde_json::json!({
        "prompt": prompt,
        "image": image,
    });

    match client.post("http://localhost:8008/inference")
        .json(&request_body)
        .send()
        .await {
            Ok(res) => {
                if res.status().is_success() {
                    let response_text = res.text().await.map_err(|e| format!("Failed to read response text: {}", e))?;
                    Ok(response_text)
                } else {
                    Err(format!("Failed to get a successful response: {}", res.status()))
                }
            },
            Err(e) => Err(format!("Failed to send request: {}", e)),
        }
}