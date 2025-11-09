use crate::setup::{get_ocr_text_detection_model_path, get_ocr_text_recognition_model_path};
use image::DynamicImage;
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tauri::AppHandle;

/// OCR result containing the extracted text and processing time
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrResult {
  pub text: String,
  pub processing_time_ms: u64,
}

/// OCR service for text extraction from images
pub struct OcrService;

impl OcrService {
  /// Load image from byte array
  pub fn load_image_from_bytes(image_data: &[u8]) -> Result<DynamicImage, String> {
    image::load_from_memory(image_data)
      .map_err(|e| format!("Failed to load image from bytes: {}", e))
  }

  /// Create OCR engine with loaded models
  pub async fn create_ocr_engine(app_handle: &AppHandle) -> Result<OcrEngine, String> {
    // Get model paths
    let detection_model_path = get_ocr_text_detection_model_path(app_handle.clone())
      .map_err(|e| format!("Failed to get detection model path: {}", e))?;

    let recognition_model_path = get_ocr_text_recognition_model_path(app_handle.clone())
      .map_err(|e| format!("Failed to get recognition model path: {}", e))?;

    // Check if model files exist
    if !detection_model_path.exists() {
      return Err(format!(
        "Text detection model not found at: {}",
        detection_model_path.display()
      ));
    }

    if !recognition_model_path.exists() {
      return Err(format!(
        "Text recognition model not found at: {}",
        recognition_model_path.display()
      ));
    }

    log::info!(
      "[OCR] Loading models from:\n  Detection: {}\n  Recognition: {}",
      detection_model_path.display(),
      recognition_model_path.display()
    );

    // Load models using rten
    let detection_model = Model::load_file(&detection_model_path)
      .map_err(|e| format!("Failed to load detection model: {}", e))?;

    let recognition_model = Model::load_file(&recognition_model_path)
      .map_err(|e| format!("Failed to load recognition model: {}", e))?;

    // Create OCR engine parameters
    let engine_params = OcrEngineParams {
      detection_model: Some(detection_model),
      recognition_model: Some(recognition_model),
      ..Default::default()
    };

    OcrEngine::new(engine_params).map_err(|e| format!("Failed to create OCR engine: {}", e))
  }

  /// Extract text from image using the OCR engine
  pub async fn extract_text_from_image(
    engine: &OcrEngine,
    image: &DynamicImage,
  ) -> Result<String, String> {
    // Convert image to RGB format if needed
    let rgb_image = image.to_rgb8();
    let (width, height) = rgb_image.dimensions();

    // Create ImageSource from image bytes and dimensions
    let image_source = ImageSource::from_bytes(rgb_image.as_raw(), (width, height))
      .map_err(|e| format!("Failed to create image source: {}", e))?;

    // Prepare input for OCR
    let ocr_input = engine
      .prepare_input(image_source)
      .map_err(|e| format!("Failed to prepare OCR input: {}", e))?;

    // Use the convenient get_text method which handles the full pipeline
    let extracted_text = engine
      .get_text(&ocr_input)
      .map_err(|e| format!("OCR processing failed: {}", e))?;

    Ok(extracted_text.trim().to_string())
  }
}

/// Process an image and extract text using OCR
#[tauri::command]
pub async fn process_image(
  app_handle: AppHandle,
  image_data: Vec<u8>,
) -> Result<OcrResult, String> {
  let start_time = Instant::now();

  log::info!(
    "[OCR] Starting OCR processing for image ({} bytes)",
    image_data.len()
  );

  // Load the image from bytes
  let image = OcrService::load_image_from_bytes(&image_data)?;

  // Create OCR engine with models
  let engine = OcrService::create_ocr_engine(&app_handle).await?;

  // Process the image
  let text = OcrService::extract_text_from_image(&engine, &image).await?;

  let processing_time = start_time.elapsed();

  log::info!(
    "[OCR] Processing completed in {}ms",
    processing_time.as_millis(),
  );

  Ok(OcrResult {
    text,
    processing_time_ms: processing_time.as_millis() as u64,
  })
}

/// Process an image file and extract text using OCR
#[tauri::command]
pub async fn process_image_from_file(
  app_handle: AppHandle,
  file_path: String,
) -> Result<OcrResult, String> {
  let start_time = Instant::now();

  log::info!(
    "[OCR] Starting OCR processing for image file: {}",
    file_path
  );

  // Load the image from file path
  let image = image::open(&file_path)
    .map_err(|e| format!("Failed to load image from file '{}': {}", file_path, e))?;

  // Create OCR engine with models
  let engine = OcrService::create_ocr_engine(&app_handle).await?;

  // Process the image
  let text = OcrService::extract_text_from_image(&engine, &image).await?;

  let processing_time = start_time.elapsed();

  log::info!(
    "[OCR] Processing completed in {}ms from file: {}",
    processing_time.as_millis(),
    file_path
  );

  Ok(OcrResult {
    text,
    processing_time_ms: processing_time.as_millis() as u64,
  })
}

/// Check if OCR models are available
#[tauri::command]
pub fn check_ocr_models_available(app_handle: AppHandle) -> Result<bool, String> {
  let detection_path = get_ocr_text_detection_model_path(app_handle.clone())?;
  let recognition_path = get_ocr_text_recognition_model_path(app_handle)?;

  Ok(detection_path.exists() && recognition_path.exists())
}
