use serde_json::Value;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use std::fs;

//TODO: Create a constants module
const EMBEDDING_DIR: &str = "models/embedding";


/// Tauri command to generate an embedding for a given prompt using the managed model.
#[tauri::command]
pub async fn get_embedding(
    app_handle: tauri::AppHandle,
    prompt: String
) -> Result<Value, String> {
    println!("[embedding] Generating embedding for prompt: \"{}\"", prompt);

    // Get cache dir for embedding model
    let app_data_path = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data directory: {}", e))?;

    let model_path = app_data_path.join(EMBEDDING_DIR);
    if !model_path.exists() {
        return Err(format!(
            "Model path does not exist: {:?}. Please ensure the model is initialized.",
            model_path
        ));
    }
    println!("[embedding] Embedding model path: {:?}", model_path);
    
    let model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::AllMiniLML6V2)
            .with_cache_dir(model_path)
            .with_show_download_progress(true),
    ).map_err(|e| format!("Failed to initialize embedding model: {}", e))?;

    // Prepare the input document array
    let documents = vec![prompt.as_str()]; // fastembed expects &str slices

    let embeddings = model.embed(documents, None).map_err(|e| {
        format!("Failed to generate embeddings: {}", e)
    })?;

    if embeddings.is_empty() {
        return Err("Embedding generation returned no results.".to_string());
    }

    // Assuming we only processed one document, take the first embedding
    // Convert Vec<f32> to serde_json::Value (Array of Numbers)
    let embedding_json: Vec<Value> = embeddings[0]
        .iter()
        .map(|&f| serde_json::Number::from_f64(f as f64).map(Value::Number))
        .collect::<Option<Vec<Value>>>()
        .ok_or("Failed to convert embedding floats to JSON numbers".to_string())?;

    Ok(Value::Array(embedding_json))
}