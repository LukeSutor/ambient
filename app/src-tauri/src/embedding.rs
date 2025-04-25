use serde_json::Value;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;

/// Calls the embedding sidecar process and returns its raw stdout.
async fn call_embedding_sidecar_internal(
    app_handle: tauri::AppHandle,
    model: String,
    prompt: String,
) -> Result<String, String> {
    println!(
        "[embedding] Calling sidecar with model: {}, prompt: {}",
        model, prompt
    );

    let shell = app_handle.shell();
    let output = shell
        .sidecar("embedding") // Assumes "embedding" is defined in tauri.conf.json
        .map_err(|e| format!("Failed to get sidecar command: {}", e))?
        .args([
            "-m",
            &model,
            "-p",
            &prompt,
            "--embd-output-format",
            "json",
            "--log-prefix",
        ])
        .output()
        .await
        .map_err(|e| {
            println!("[embedding] Failed to execute sidecar command: {}", e);
            format!("Sidecar execution failed: {}", e)
        })?;

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout).map_err(|e| {
            println!("[embedding] Failed to decode stdout: {}", e);
            format!("Failed to decode stdout: {}", e)
        })?;
        // Don't print the full stdout here as it can be very long (contains logs + JSON)
        println!("[embedding] Sidecar executed successfully.");
        Ok(stdout)
    } else {
        let stderr = String::from_utf8(output.stderr).map_err(|e| {
            println!("[embedding] Failed to decode stderr: {}", e);
            format!("Failed to decode stderr: {}", e)
        })?;
        println!(
            "[embedding] Sidecar execution failed. Status: {:?}, Stderr:\n{}",
            output.status, stderr
        );
        Err(format!(
            "Sidecar execution failed with status {:?}: {}",
            output.status, stderr
        ))
    }
}

/// Parses the raw output string from the embedding sidecar to extract the JSON embedding.
fn parse_embedding_output(output_string: &str) -> Result<Value, String> {
    // Find the start of the JSON block (first '{' on a line potentially preceded by whitespace)
    let json_start = output_string.find('{').ok_or_else(|| {
        println!("[embedding] Could not find start of JSON ('{{') in output.");
        "Could not find start of JSON ('{') in output.".to_string()
    })?;

    // Find the end of the JSON block (last '}')
    let json_end = output_string.rfind('}').ok_or_else(|| {
        println!("[embedding] Could not find end of JSON ('}}') in output.");
        "Could not find end of JSON ('}') in output.".to_string()
    })?;

    if json_start >= json_end {
        return Err("JSON start marker found after or at end marker.".to_string());
    }

    // Extract the JSON substring
    let json_substring = &output_string[json_start..=json_end];

    // Parse the JSON substring
    serde_json::from_str(json_substring).map_err(|e| {
        println!("[embedding] Failed to parse JSON: {}", e);
        // Optionally include a snippet of the problematic JSON for debugging
        // let snippet = json_substring.chars().take(100).collect::<String>();
        // format!("Failed to parse JSON: {} (starts with: {}...)", e, snippet)
        format!("Failed to parse JSON: {}", e)
    })
}

/// Tauri command to generate an embedding for a given prompt using a specified model.
/// Calls the sidecar and parses the JSON output.
#[tauri::command]
pub async fn get_embedding(
    app_handle: tauri::AppHandle,
    model: String,
    prompt: String,
) -> Result<Value, String> {
    let raw_output = call_embedding_sidecar_internal(app_handle, model, prompt).await?;
    parse_embedding_output(&raw_output)
}
