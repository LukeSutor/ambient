use crate::constants::*;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use reqwest::Client;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::{fs::File, io::Write};
use tauri::{Emitter, Manager};
use tokio_stream::StreamExt;

/// Objects for download progress
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadStarted {
  id: u64,
  content_length: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress {
  id: u64,
  total_progress: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadFinished {
  id: u64,
}

/// Setup function to download vlm and fastembed models
#[tauri::command]
pub async fn setup(app_handle: tauri::AppHandle) -> Result<String, String> {
  println!("[setup] Starting model setup...");

  // Download the vlm files
  if let Err(e) = initialize_vlm(app_handle.clone()).await {
    eprintln!("[setup] VLM initialization failed: {}", e);
    return Err(format!("Failed to initialize VLM: {}", e));
  }
  println!("[setup] VLM initialization successful.");

  // Download the fastembed files
  // Note: FastEmbed handles its own download internally. Progress is shown in console, not via events.
  if let Err(e) = initialize_fastembed(app_handle.clone()).await {
    eprintln!("[setup] FastEmbed initialization failed: {}", e);
    return Err(format!("Failed to initialize FastEmbed: {}", e));
  }
  println!("[setup] FastEmbed initialization successful.");

  Ok("Setup completed successfully.".to_string()) // More accurate success message
}

/// Setup function to download the vlm from huggingface
async fn initialize_vlm(app_handle: tauri::AppHandle) -> Result<String, String> {
  // Get cache dir for vlm
  let app_data_path = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| format!("Could not resolve app data directory: {}", e))?;

  let model_path = app_data_path.join(VLM_DIR);
  fs::create_dir_all(&model_path) // Create the specific VLM directory
    .map_err(|e| format!("Failed to create VLM model directory: {}", e))?;

  for (id, url, out_path) in [(1, TEXT_LINK, TEXT_FILE), (2, MMPROJ_LINK, MMPROJ_FILE)] {
    let full_out_path = model_path.join(out_path);
    println!("[setup] Downloading model {} to {:?}", id, full_out_path);

    if full_out_path.exists() {
      println!("[setup] Model {} already exists. Skipping download.", id);
      // Optionally emit finished event here if needed by frontend logic
      if let Err(e) = app_handle.emit("download-finished", DownloadFinished { id }) {
        eprintln!("[setup] Failed to emit skip event for model {}: {}", id, e);
      }
      continue;
    }

    let client = Client::new();
    let response = client.get(url).send().await.map_err(|e| e.to_string())?;
    let total_size = response
      .content_length()
      .ok_or_else(|| "Failed to get content length".to_string())?;

    // Send start update
    if let Err(e) = app_handle.emit(
      "download-started",
      DownloadStarted {
        id: id,
        content_length: total_size,
      },
    ) {
      eprintln!("Failed to emit event: {}", e);
    }

    let mut file = File::create(&full_out_path)
      .map_err(|e| format!("Failed to create file for model {}: {}", id, e))?;
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    // Process the stream of chunks
    while let Some(chunk) = stream.next().await {
      let chunk_data = chunk.map_err(|e| e.to_string())?;
      file.write_all(&chunk_data).map_err(|e| e.to_string())?;
      downloaded += chunk_data.len() as u64;

      // Send progress update
      if let Err(e) = app_handle.emit(
        "download-progress",
        DownloadProgress {
          id: id,
          total_progress: downloaded,
        },
      ) {
        eprintln!("Failed to emit progress event: {}", e);
      }
    }

    // Send completion update
    if let Err(e) = app_handle.emit("download-finished", DownloadFinished { id: id }) {
      eprintln!("Failed to emit finished event: {}", e);
    }
  }
  Ok("VLM models initialized successfully.".to_string())
}

/// Setup function to download the fastembed model from huggingface
async fn initialize_fastembed(app_handle: tauri::AppHandle) -> Result<String, String> {
  // Get cache dir for embedding model
  let app_data_path = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| format!("Could not resolve app data directory: {}", e))?;
  fs::create_dir_all(&app_data_path)
    .map_err(|e| format!("Failed to create app data directory: {}", e))?;
  let model_path = app_data_path.join(EMBEDDING_DIR);
  println!("[embedding] Embedding model path: {:?}", model_path);

  let model = TextEmbedding::try_new(
    InitOptions::new(EmbeddingModel::AllMiniLML6V2)
      .with_cache_dir(model_path)
      .with_show_download_progress(true),
  )
  .map_err(|e| format!("Failed to initialize embedding model: {}", e))?;

  // Test if the model works by generating embeddings for sample documents
  let documents = vec!["passage: Hello, World!"];

  let embeddings = model
    .embed(documents, None)
    .map_err(|e| format!("Failed to generate embeddings: {}", e))?;

  if embeddings.len() != 1 {
    return Err(format!(
      "Expected exactly 1 embedding, but got {}.",
      embeddings.len()
    ));
  }

  Ok("Embedding model initialized and tested successfully.".to_string())
}

/// Gets the path of the VLM text model file.
#[tauri::command]
pub fn get_vlm_text_model_path(app_handle: tauri::AppHandle) -> Result<PathBuf, String> {
  let app_data_path = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| format!("App data directory could not be resolved: {}", e))?;
  let vlm_models_dir = app_data_path.join(VLM_DIR);
  Ok(vlm_models_dir.join(TEXT_FILE))
}

/// Gets the path of the VLM mmproj model file.
#[tauri::command]
pub fn get_vlm_mmproj_model_path(app_handle: tauri::AppHandle) -> Result<PathBuf, String> {
  let app_data_path = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| format!("App data directory could not be resolved: {}", e))?;
  let vlm_models_dir = app_data_path.join(VLM_DIR);
  Ok(vlm_models_dir.join(MMPROJ_FILE))
}

/// Checks if the VLM text model file is downloaded.
#[tauri::command]
pub fn check_vlm_text_model_download(app_handle: tauri::AppHandle) -> Result<bool, String> {
  match get_vlm_text_model_path(app_handle) {
    Ok(path) => Ok(path.exists()),
    Err(e) => {
      eprintln!("[check_setup] Failed to get VLM text model path: {}", e);
      // If we can't get the path, treat it as not downloaded, but don't error out the check itself
      Ok(false)
    }
  }
}

/// Checks if the VLM mmproj model file is downloaded.
#[tauri::command]
pub fn check_vlm_mmproj_model_download(app_handle: tauri::AppHandle) -> Result<bool, String> {
  match get_vlm_mmproj_model_path(app_handle) {
    Ok(path) => Ok(path.exists()),
    Err(e) => {
      eprintln!("[check_setup] Failed to get VLM mmproj model path: {}", e);
      // If we can't get the path, treat it as not downloaded, but don't error out the check itself
      Ok(false)
    }
  }
}

/// Gets the directory path intended for the FastEmbed models.
/// Note: FastEmbed manages specific files within this directory.
#[tauri::command]
pub fn get_fastembed_model_path(app_handle: tauri::AppHandle) -> Result<PathBuf, String> {
  // Get the application data directory
  let app_data_path = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| format!("App data directory could not be resolved: {}", e))?;

  // Construct the specific directory for embedding models
  let embedding_models_dir = app_data_path.join(EMBEDDING_DIR);

  Ok(embedding_models_dir)
}

/// Checks if the FastEmbed model (specifically AllMiniLML6V2) appears to be downloaded.
#[tauri::command]
pub fn check_fastembed_model_download(app_handle: tauri::AppHandle) -> Result<bool, String> {
  println!("[check_setup] Checking FastEmbed model download status...");
  let model_dir_result = get_fastembed_model_path(app_handle);
  let model_dir = match model_dir_result {
    Ok(dir) => dir,
    Err(e) => {
      // If we can't even get the path, assume not downloaded or setup error
      eprintln!("[check_setup] Failed to get FastEmbed model path: {}", e);
      return Ok(false);
    }
  };

  // Check if the directory exists.
  if !model_dir.exists() || !model_dir.is_dir() {
    println!(
      "[check_setup] FastEmbed model directory does not exist or is not a directory: {:?}",
      model_dir
    );
    return Ok(false);
  }

  // Check if the directory is empty.
  match fs::read_dir(&model_dir) {
    Ok(mut entries) => {
      if entries.next().is_none() {
        // Directory exists but is empty
        println!(
          "[check_setup] FastEmbed model directory exists but is empty: {:?}",
          model_dir
        );
        Ok(false)
      } else {
        // Directory exists and is not empty
        println!(
          "[check_setup] FastEmbed model directory exists and is not empty: {:?}",
          model_dir
        );
        Ok(true)
      }
    }
    Err(e) => {
      // Error reading directory contents
      eprintln!(
        "[check_setup] Failed to read FastEmbed model directory contents: {}",
        e
      );
      // Treat error reading contents as potentially incomplete setup
      Ok(false)
    }
  }
}

/// General function to check if all required models (VLM and FastEmbed) are downloaded.
#[tauri::command]
pub fn check_setup_complete(app_handle: tauri::AppHandle) -> Result<bool, String> {
  println!("[check_setup] Checking overall setup completeness...");

  // Check VLM text model
  let vlm_text_downloaded = match check_vlm_text_model_download(app_handle.clone()) {
    Ok(downloaded) => downloaded,
    Err(e) => {
      eprintln!(
        "[check_setup] Error checking VLM text model download status: {}",
        e
      );
      // Propagate the error if the check itself failed unexpectedly
      return Err(format!("Error checking VLM text model status: {}", e));
    }
  };

  if !vlm_text_downloaded {
    println!("[check_setup] VLM text model not downloaded. Setup incomplete.");
    return Ok(false);
  }
  println!("[check_setup] VLM text model appears to be downloaded.");

  // Check VLM mmproj model
  let vlm_mmproj_downloaded = match check_vlm_mmproj_model_download(app_handle.clone()) {
    Ok(downloaded) => downloaded,
    Err(e) => {
      eprintln!(
        "[check_setup] Error checking VLM mmproj model download status: {}",
        e
      );
      return Err(format!("Error checking VLM mmproj model status: {}", e));
    }
  };

  if !vlm_mmproj_downloaded {
    println!("[check_setup] VLM mmproj model not downloaded. Setup incomplete.");
    return Ok(false);
  }
  println!("[check_setup] VLM mmproj model appears to be downloaded.");

  // Check FastEmbed model
  let fastembed_downloaded = match check_fastembed_model_download(app_handle) {
    Ok(downloaded) => downloaded,
    Err(e) => {
      eprintln!(
        "[check_setup] Error checking FastEmbed download status: {}",
        e
      );
      return Err(format!("Error checking FastEmbed status: {}", e)); // Propagate error
    }
  };

  if !fastembed_downloaded {
    println!("[check_setup] FastEmbed model not downloaded. Setup incomplete.");
    return Ok(false);
  }
  println!("[check_setup] FastEmbed model appears to be downloaded.");

  // If all checks passed
  println!("[check_setup] All models appear to be downloaded. Setup complete.");
  Ok(true)
}
