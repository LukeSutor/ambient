use crate::constants::{
  EMBEDDING_DIR, EMBEDDING_FILE, EMBEDDING_LINK, EMBEDDING_TOKENIZER_FILE, EMBEDDING_TOKENIZER_LINK, MMPROJ_FILE, MMPROJ_LINK, OCR_DIR, TEXT_DETECTION_FILE,
  TEXT_DETECTION_LINK, TEXT_FILE, TEXT_LINK, TEXT_RECOGNITION_FILE, TEXT_RECOGNITION_LINK,
  VLM_DIR,
};
use crate::events::{
  emitter::emit,
  types::{DOWNLOAD_INFORMATION, DownloadInformationEvent, DOWNLOAD_STARTED, DownloadStartedEvent, DOWNLOAD_PROGRESS, DownloadProgressEvent, DOWNLOAD_FINISHED, DownloadFinishedEvent},
};
use crate::models::llm::server::spawn_llama_server;
use reqwest::Client;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{fs::File, io::Write};
use tauri::{AppHandle, Emitter, Manager};
use tokio_stream::StreamExt;

/// Global lock to prevent multiple concurrent setup processes
static SETUP_RUNNING: AtomicBool = AtomicBool::new(false);

/// Struct representing a download item
#[derive(Clone)]
struct DownloadItem {
  id: u64,
  url: &'static str,
  out_file: PathBuf,
}

impl DownloadItem {
  pub fn new(
    id: u64,
    url: &'static str,
    out_file: PathBuf,
  ) -> Self {
    DownloadItem {
      id,
      url,
      out_file,
    }
  }

  pub async fn download(&self) -> Result<(), String> {
    let parent_dir = self.out_file
      .parent()
      .ok_or_else(|| "Failed to get parent directory".to_string())?;
    fs::create_dir_all(&parent_dir) // Create the specific model directory
      .map_err(|e| format!("Failed to create model directory: {}", e))?;
      
    if self.out_file.exists() {
      log::info!("[setup] Model {} already exists. Skipping download.", self.id);
      // Optionally emit finished event here if needed by frontend logic
      if let Err(e) = emit(DOWNLOAD_FINISHED, DownloadFinishedEvent { id: self.id }) {
        log::error!("[setup] Failed to emit skip event for model {}: {}", self.id, e);
      }
      return Ok(());
    }
      
    let client = Client::new();
    let response = client.get(self.url).send().await.map_err(|e| e.to_string())?;

    // Send start update
    log::info!(
      "[setup] Downloading model {}",
      self.id,
    );

    if let Err(e) = emit(
      DOWNLOAD_STARTED,
      DownloadStartedEvent { id: self.id },
    ) {
      log::error!("Failed to emit event: {}", e);
    }

    let mut file = File::create(&self.out_file)
      .map_err(|e| format!("Failed to create file for model {}: {}", self.id, e))?;
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    // Process the stream of chunks
    while let Some(chunk) = stream.next().await {
      let chunk_data = chunk.map_err(|e| e.to_string())?;
      file.write_all(&chunk_data).map_err(|e| e.to_string())?;
      downloaded += chunk_data.len() as u64;

      // Send progress update
      if let Err(e) = emit(
        DOWNLOAD_PROGRESS,
        DownloadProgressEvent {
          id: self.id,
          total_progress: downloaded,
        },
      ) {
        log::error!("Failed to emit progress event: {}", e);
      }
    }
    // Send completion update
    if let Err(e) = emit(
      DOWNLOAD_FINISHED,
      DownloadFinishedEvent { id: self.id },
    ) {
      log::error!("Failed to emit finished event: {}", e);
    }
    Ok(())
  }
}

/// Create all download items
async fn create_needed_download_items(
  app_handle: &AppHandle,
) -> Result<Vec<DownloadItem>, String> {
  let app_data_path = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| format!("Could not resolve app data directory: {}", e))?;

  let vlm_model_path = app_data_path.join(VLM_DIR);
  let ocr_model_path = app_data_path.join(OCR_DIR);
  let embedding_model_path = app_data_path.join(EMBEDDING_DIR);

  let mut items = Vec::new();
  for (download_link, model_dir, file_name) in &[
    (TEXT_LINK, &vlm_model_path, TEXT_FILE),
    (MMPROJ_LINK, &vlm_model_path, MMPROJ_FILE),
    (EMBEDDING_LINK, &embedding_model_path, EMBEDDING_FILE),
    (EMBEDDING_TOKENIZER_LINK, &embedding_model_path, EMBEDDING_TOKENIZER_FILE),
    (TEXT_DETECTION_LINK, &ocr_model_path, TEXT_DETECTION_FILE),
    (TEXT_RECOGNITION_LINK, &ocr_model_path, TEXT_RECOGNITION_FILE),
  ] {
    // Only add download item if file does not already exist
    let full_out_path = model_dir.join(file_name);
    if !full_out_path.exists() {
      let id = items.len() as u64 + 1;
      items.push(DownloadItem::new(
        id,
        *download_link,
        full_out_path,
      ));
    }
  }

  Ok(items)
}

/// Get total content length of all download items
async fn get_total_content_length(items: Vec<DownloadItem>) -> Result<u64, String> {
  let client = Client::new();
  let mut total_size: u64 = 0;

  for item in items {
    let response = client
      .head(item.url)
      .send()
      .await
      .map_err(|e| e.to_string())?;
    let content_length = response
      .headers()
      .get(reqwest::header::CONTENT_LENGTH)
      .and_then(|val| val.to_str().ok())
      .and_then(|s| s.parse::<u64>().ok())
      .ok_or_else(|| format!("Failed to get content length for URL: {}", item.url))?;
    total_size += content_length;
  }

  Ok(total_size)
}

/// Setup function to get download information
#[tauri::command]
pub async fn get_setup_download_info(
  app_handle: AppHandle,
) -> Result<DownloadInformationEvent, String> {
  log::info!("[setup] Fetching download information...");

  // Get download items
  let download_items = create_needed_download_items(&app_handle).await?;

  // Get total content length
  let total_content_length = get_total_content_length(download_items.clone()).await?;

  Ok(DownloadInformationEvent {
    n_items: download_items.len() as u64,
    content_length: total_content_length,
  })
}

/// Setup function to download all necessary models
#[tauri::command]
pub async fn setup(app_handle: AppHandle) -> Result<String, String> {
  // Check if setup is already running
  if SETUP_RUNNING.swap(true, Ordering::SeqCst) {
    log::warn!("[setup] Setup already in progress, ignoring request");
    return Err("Setup is already in progress.".to_string());
  }

  // Use a helper function or block to ensure we always reset the flag
  let setup_result = async {
    log::info!("[setup] Starting model setup...");

    // Get download items
    let download_items = create_needed_download_items(&app_handle).await?;

    // Get total content length
    let total_content_length = get_total_content_length(download_items.clone()).await?;

    // Emit total download information
    if let Err(e) = emit(
      DOWNLOAD_INFORMATION,
      DownloadInformationEvent {
        n_items: download_items.len() as u64,
        content_length: total_content_length,
      },
    ) {
      log::error!("Failed to emit download information event: {}", e);
    }

    // Download each item sequentially
    for item in download_items {
      if let Err(e) = item.download().await {
        log::error!("[setup] Failed to download model {}: {}", item.id, e);
        return Err(format!("Failed to download model {}: {}", item.id, e));
      }
    }

    // Spawn the llama server but don't block on it
    let app_handle_clone = app_handle.clone();
    let _ = tokio::spawn(async move {
      if let Err(e) = spawn_llama_server(app_handle_clone).await {
        log::error!("[setup] Failed to spawn LLaMA server: {}", e);
      }
    });

    // Emit auth changed event
    app_handle
      .emit("auth_changed", {})
      .map_err(|e| format!("Failed to emit auth-changed event: {}", e))?;

    Ok("Setup completed successfully.".to_string())
  }.await;

  // Reset the flag
  SETUP_RUNNING.store(false, Ordering::SeqCst);

  setup_result
}

/// Gets the path of the VLM text model file
pub fn get_vlm_text_model_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
  let app_data_path = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| format!("App data directory could not be resolved: {}", e))?;
  let vlm_models_dir = app_data_path.join(VLM_DIR);
  Ok(vlm_models_dir.join(TEXT_FILE))
}

/// Gets the path of the VLM mmproj model file
pub fn get_vlm_mmproj_model_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
  let app_data_path = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| format!("App data directory could not be resolved: {}", e))?;
  let vlm_models_dir = app_data_path.join(VLM_DIR);
  Ok(vlm_models_dir.join(MMPROJ_FILE))
}

/// Checks if the VLM text model file is downloaded
pub fn check_vlm_text_model_download(app_handle: &AppHandle) -> Result<bool, String> {
  match get_vlm_text_model_path(&app_handle) {
    Ok(path) => Ok(path.exists()),
    Err(e) => {
      log::error!("[check_setup] Failed to get VLM text model path: {}", e);
      // If we can't get the path, treat it as not downloaded, but don't error out the check itself
      Ok(false)
    }
  }
}

/// Checks if the VLM mmproj model file is downloaded
pub fn check_vlm_mmproj_model_download(app_handle: &AppHandle) -> Result<bool, String> {
  match get_vlm_mmproj_model_path(&app_handle) {
    Ok(path) => Ok(path.exists()),
    Err(e) => {
      log::error!("[check_setup] Failed to get VLM mmproj model path: {}", e);
      // If we can't get the path, treat it as not downloaded, but don't error out the check itself
      Ok(false)
    }
  }
}

/// Gets the path of the OCR text detection model file
pub fn get_ocr_text_detection_model_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
  let app_data_path = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| format!("App data directory could not be resolved: {}", e))?;
  let ocr_models_dir = app_data_path.join(OCR_DIR);
  Ok(ocr_models_dir.join(TEXT_DETECTION_FILE))
}

/// Gets the path of the OCR text recognition model file
pub fn get_ocr_text_recognition_model_path(
  app_handle: &AppHandle,
) -> Result<PathBuf, String> {
  let app_data_path = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| format!("App data directory could not be resolved: {}", e))?;
  let ocr_models_dir = app_data_path.join(OCR_DIR);
  Ok(ocr_models_dir.join(TEXT_RECOGNITION_FILE))
}

/// Checks if the OCR text detection model file exists
pub fn check_ocr_text_detection_model_download(
  app_handle: &AppHandle,
) -> Result<bool, String> {
  match get_ocr_text_detection_model_path(app_handle) {
    Ok(path) => Ok(path.exists()),
    Err(e) => {
      log::error!(
        "[check_setup] Failed to get OCR text detection model path: {}",
        e
      );
      Ok(false)
    }
  }
}

/// Checks if the OCR text recognition model file exists
pub fn check_ocr_text_recognition_model_download(
  app_handle: &AppHandle,
) -> Result<bool, String> {
  match get_ocr_text_recognition_model_path(app_handle) {
    Ok(path) => Ok(path.exists()),
    Err(e) => {
      log::error!(
        "[check_setup] Failed to get OCR text recognition model path: {}",
        e
      );
      Ok(false)
    }
  }
}

/// Gets the path of the embedding model file
pub fn get_embedding_model_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
  let app_data_path = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| format!("App data directory could not be resolved: {}", e))?;
  let embedding_models_dir = app_data_path.join(EMBEDDING_DIR);
  Ok(embedding_models_dir.join(EMBEDDING_FILE))
}

pub fn get_embedding_tokenizer_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
  let app_data_path = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| format!("App data directory could not be resolved: {}", e))?;
  let embedding_models_dir = app_data_path.join(EMBEDDING_DIR);
  Ok(embedding_models_dir.join(crate::constants::EMBEDDING_TOKENIZER_FILE))
}

/// Checks if the embedding model file is downloaded
pub fn check_embedding_model_download(app_handle: &AppHandle) -> Result<bool, String> {
  let model_path = get_embedding_model_path(app_handle)?;
  let tokenizer_path = get_embedding_tokenizer_path(app_handle)?;
  Ok(model_path.exists() && model_path.is_file() && 
     tokenizer_path.exists() && tokenizer_path.is_file())
}

/// General function to check if all required models (VLM and FastEmbed) are downloaded.
#[tauri::command]
pub fn check_setup_complete(app_handle: tauri::AppHandle) -> Result<bool, String> {
  // Check VLM text model
  let vlm_text_downloaded = match check_vlm_text_model_download(&app_handle) {
    Ok(downloaded) => downloaded,
    Err(e) => {
      log::error!(
        "[check_setup] Error checking VLM text model download status: {}",
        e
      );
      // Propagate the error if the check itself failed unexpectedly
      return Err(format!("Error checking VLM text model status: {}", e));
    }
  };

  if !vlm_text_downloaded {
    log::warn!("[check_setup] VLM text model not downloaded. Setup incomplete.");
    return Ok(false);
  }

  // Check VLM mmproj model
  let vlm_mmproj_downloaded = match check_vlm_mmproj_model_download(&app_handle) {
    Ok(downloaded) => downloaded,
    Err(e) => {
      log::error!(
        "[check_setup] Error checking VLM mmproj model download status: {}",
        e
      );
      return Err(format!("Error checking VLM mmproj model status: {}", e));
    }
  };

  if !vlm_mmproj_downloaded {
    log::warn!("[check_setup] VLM mmproj model not downloaded. Setup incomplete.");
    return Ok(false);
  }

  // Check OCR text detection model
  let ocr_text_detection_downloaded = match check_ocr_text_detection_model_download(&app_handle) {
    Ok(downloaded) => downloaded,
    Err(e) => {
      log::error!(
        "[check_setup] Error checking OCR text detection model download status: {}",
        e
      );
      return Err(format!("Error checking OCR text detection model status: {}", e));
    }
  };

  if !ocr_text_detection_downloaded {
    log::warn!(
      "[check_setup] OCR text detection model not downloaded. Setup incomplete."
    );
    return Ok(false);
  }

  // Check OCR text recognition model
  let ocr_text_recognition_downloaded = match check_ocr_text_recognition_model_download(&app_handle) {
    Ok(downloaded) => downloaded,
    Err(e) => {
      log::error!(
        "[check_setup] Error checking OCR text recognition model download status: {}",
        e
      );
      return Err(format!("Error checking OCR text recognition model status: {}", e));
    }
  };

  if !ocr_text_recognition_downloaded {
    log::warn!(
      "[check_setup] OCR text recognition model not downloaded. Setup incomplete."
    );
    return Ok(false);
  }

  // Check embedding model
  let embedding_model_downloaded = match check_embedding_model_download(&app_handle) {
    Ok(downloaded) => downloaded,
    Err(e) => {
      log::error!(
        "[check_setup] Error checking embedding model download status: {}",
        e
      );
      return Err(format!("Error checking embedding model status: {}", e));
    }
  };

  if !embedding_model_downloaded {
    log::warn!("[check_setup] Embedding model not downloaded. Setup incomplete.");
    return Ok(false);
  }

  Ok(true)
}
