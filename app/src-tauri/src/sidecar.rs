use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::Manager;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

// Function to call the main sidecar with arguments and get the output
#[tauri::command]
pub async fn call_main_sidecar(app_handle: tauri::AppHandle, image_path: String, prompt: String) -> Result<String, String> {
    println!("[tauri] Received command to call main sidecar with image: {} and prompt: {}", image_path, prompt);

    let shell = app_handle.shell();
    // Assuming your sidecar executable is named "main". Adjust if necessary.
    // If "main" is registered as a sidecar in tauri.conf.json, use shell.sidecar("main") instead.
    let output = shell.command("main") // Or shell.sidecar("main") if configured
        .args([&image_path, &prompt])
        .output()
        .await
        .map_err(|e| {
            println!("[tauri] Failed to execute sidecar command: {}", e);
            e.to_string()
        })?;

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout).map_err(|e| {
            println!("[tauri] Failed to decode stdout: {}", e);
            e.to_string()
        })?;
        println!("[tauri] Sidecar executed successfully. Output:\n{}", stdout);
        Ok(stdout)
    } else {
        let stderr = String::from_utf8(output.stderr).map_err(|e| {
            println!("[tauri] Failed to decode stderr: {}", e);
            e.to_string()
        })?;
        println!("[tauri] Sidecar execution failed. Status: {:?}, Stderr:\n{}", output.status, stderr);
        Err(format!(
            "Sidecar execution failed with status {:?}: {}",
            output.status, stderr
        ))
    }
}
