use std::sync::Arc;
use tauri::Manager;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use tokio::sync::Mutex;
use crate::setup; // Import setup functions


/// Internal function to call the VLM (llama) sidecar process and return raw output.
async fn call_vlm_sidecar_internal(
    app_handle: tauri::AppHandle,
    image: String,
    prompt: String,
) -> Result<String, String> {
    // Get model paths using functions from setup.rs
    let model_path = setup::get_vlm_text_model_path(app_handle.clone())?;
    let mmproj_path = setup::get_vlm_mmproj_model_path(app_handle.clone())?;

    // Convert paths to strings, handling potential errors if paths are not valid UTF-8
    let model_str = model_path.to_str()
        .ok_or_else(|| format!("Model path is not valid UTF-8: {:?}", model_path))?
        .to_string();
    let mmproj_str = mmproj_path.to_str()
        .ok_or_else(|| format!("Mmproj path is not valid UTF-8: {:?}", mmproj_path))?
        .to_string();

    println!(
        "[vlm] Calling sidecar with image: {}, prompt: {}",
        image, prompt
    );

    let shell = app_handle.shell();
    let output = shell
        .sidecar("llama")
        .map_err(|e| format!("Failed to get sidecar command: {}", e))?
        .args([
            "-m", &model_str,
            "--mmproj", &mmproj_str,
            "--image", &image,
            "-p", &prompt,
        ])
        .output()
        .await
        .map_err(|e| {
            println!("[vlm] Failed to execute sidecar command: {}", e);
            format!("Sidecar execution failed: {}", e)
        })?;

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout).map_err(|e| {
            println!("[vlm] Failed to decode stdout: {}", e);
            format!("Failed to decode stdout: {}", e)
        })?;
        println!("[vlm] Sidecar executed successfully. Raw output:\n{}", stdout);
        Ok(stdout)
    } else {
        let stderr = String::from_utf8(output.stderr).map_err(|e| {
            println!("[vlm] Failed to decode stderr: {}", e);
            format!("Failed to decode stderr: {}", e)
        })?;
        println!(
            "[vlm] Sidecar execution failed. Status: {:?}, Stderr:\n{}",
            output.status, stderr
        );
        Err(format!(
            "Sidecar execution failed with status {:?}: {}",
            output.status, stderr
        ))
    }
}

/// Parses the raw output from the VLM sidecar to extract the model's response.
fn parse_vlm_output(output_string: &str) -> Result<String, String> {
    // Find the marker line indicating the start of the actual response
    // Adjust this marker if the sidecar's logging changes
    let marker = "decoding image batch";
    if let Some(marker_pos) = output_string.find(marker) {
        // Find the end of the line containing the marker
        if let Some(line_end_pos) = output_string[marker_pos..].find('\n') {
            // The response starts after this line
            let response_start_pos = marker_pos + line_end_pos + 1;
            // Trim leading/trailing whitespace from the extracted response
            let response = output_string[response_start_pos..].trim();
            if response.is_empty() {
                Err("Extracted response is empty after parsing.".to_string())
            } else {
                Ok(response.to_string())
            }
        } else {
            // Marker found, but no newline after it? Return rest of string maybe?
            let response = output_string[marker_pos + marker.len()..].trim();
             if response.is_empty() {
                Err("Could not find newline after marker, and remaining string is empty.".to_string())
            } else {
                println!("[vlm] Warning: No newline found after marker, returning rest of string.");
                Ok(response.to_string())
            }
        }
    } else {
        println!("[vlm] Could not find response marker '{}' in output.", marker);
        Err(format!("Could not find response marker '{}' in output.", marker))
    }
}

/// Tauri command to get a response from the VLM sidecar for an image and prompt.
#[tauri::command]
pub async fn get_vlm_response(
    app_handle: tauri::AppHandle,
    image: String,
    prompt: String,
) -> Result<String, String> {
    let raw_output = call_vlm_sidecar_internal(app_handle, image, prompt).await?;
    parse_vlm_output(&raw_output)
}
