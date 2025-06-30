use crate::constants::VLM_CHAT_TEMPLATE;
use crate::setup;
use tauri_plugin_shell::ShellExt;

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
  let model_str = model_path
    .to_str()
    .ok_or_else(|| format!("Model path is not valid UTF-8: {:?}", model_path))?
    .to_string();
  let mmproj_str = mmproj_path
    .to_str()
    .ok_or_else(|| format!("Mmproj path is not valid UTF-8: {:?}", mmproj_path))?
    .to_string();

  let shell = app_handle.shell();
  let output = shell
    .sidecar("llama")
    .map_err(|e| format!("Failed to get sidecar command: {}", e))?
    .args([
      "-m",
      &model_str,
      "--mmproj",
      &mmproj_str,
      "--image",
      &image,
      "-p",
      &prompt,
      "--chat-template",
      VLM_CHAT_TEMPLATE,
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
    println!(
      "[vlm] Sidecar executed successfully. Raw output:\n{}",
      stdout
    );
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

/// Parses the raw output from the VLM sidecar to extract the model's response as JSON.
fn parse_vlm_output(output_string: &str) -> Result<serde_json::Value, String> {
  let start_marker = "<|START|>";
  let end_marker = "<|END|>";

  let start = output_string
    .find(start_marker)
    .ok_or_else(|| format!("Could not find start marker '{}' in output.", start_marker))?;
  let end = output_string
    .find(end_marker)
    .ok_or_else(|| format!("Could not find end marker '{}' in output.", end_marker))?;

  if end <= start + start_marker.len() {
    return Err("End marker found before start marker.".to_string());
  }

  let json_str = &output_string[start + start_marker.len()..end].trim();
  serde_json::from_str(json_str).map_err(|e| format!("Failed to parse JSON from VLM output: {}", e))
}

/// Tauri command to get a response from the VLM sidecar for an image and prompt.
/// Returns a serde_json::Value object.
#[tauri::command]
pub async fn get_vlm_response(
  app_handle: tauri::AppHandle,
  image: String,
  prompt: String,
) -> Result<serde_json::Value, String> {
  let raw_output = call_vlm_sidecar_internal(app_handle, image, prompt).await?;
  parse_vlm_output(&raw_output)
}
