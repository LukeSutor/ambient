use tauri_plugin_shell::ShellExt;
use tauri::{AppHandle};
use crate::setup::{get_embedding_model_path};

#[tauri::command]
pub async fn generate_embedding(app_handle: AppHandle, input: String) -> Result<Vec<f32>, String> {
    log::info!("[Embedding] Generating embedding for raw input: {}", input);

    // Apply EmbeddingGemma document-style prompt with a 'none' title.
    // Format: "title: none | text: {content}" as recommended for document embeddings
    let memory_text = input.trim();
    let prompt = format!("title: none | text: {}", memory_text);

    let model_path = get_embedding_model_path(app_handle.clone())
        .map_err(|e| format!("Failed to get embedding model path: {}", e))?;
    
    // Prepare sidecar command
    let shell = app_handle.shell();
    let sidecar_command = shell
        .sidecar("embedding")
        .map_err(|e| format!("Failed to get sidecar command: {}", e))?
        .args([
            "-m",
            model_path.to_str().ok_or("Invalid model path")?,
            "-p",
            prompt.as_str(),
            "--ctx-size",
            "2048"
        ]);

    // Run sidecar and capture output directly
    let output = sidecar_command
        .output()
        .await
        .map_err(|e| format!("Failed to execute embedding sidecar: {}", e))?;

    if !output.status.success() {
        return Err(format!("Embedding process failed with status: {:?}", output.status));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| format!("Failed to parse process output: {}", e))?;

    let embedding = parse_embedding_output(&stdout)?;

    log::info!("[Embedding] Generated embedding: {:?}", embedding);

    Ok(embedding)
}

fn parse_embedding_output(output: &str) -> Result<Vec<f32>, String> {
    // Find the line that starts with "embedding 0:"
    let embedding_line = output.lines()
        .find(|line| line.contains("embedding 0:"))
        .ok_or("Could not find embedding output line")?;
    
    // Extract everything after "embedding 0:"
    let numbers_part = embedding_line
        .split("embedding 0:")
        .nth(1)
        .ok_or("Could not extract numbers from embedding line")?;
    
    // Parse the space-separated numbers
    let embedding: Vec<f32> = numbers_part
        .split_whitespace()
        .map(|s| s.parse::<f32>().map_err(|e| format!("Failed to parse float '{}': {}", s, e)))
        .collect::<Result<Vec<f32>, _>>()?;
    
    Ok(embedding)
}