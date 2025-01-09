// Contains functions for the application data control

use screenshots::Screen;
use std::{fs::File, io::Write};
use std::fs;
use tauri::{Manager, Emitter};
use image::imageops::FilterType;
use std::path::PathBuf;
use reqwest::Client;
use serde::Serialize;
use tokio_stream::StreamExt;

// Gets the paths of the model files
#[tauri::command]
pub fn get_model_paths(app_handle: tauri::AppHandle) -> Vec<PathBuf> {
    // Get the directory where the models are saved
    let app_data_path = app_handle
        .path()
        .app_data_dir()
        .expect("App data dir could not be fetched.");
    let models_dir = app_data_path.join("models");

    // Collect the model file paths
    let text_model_path = models_dir.join("qwen2vl-2b-text.gguf");
    let vision_model_path = models_dir.join("qwen2vl-2b-vision.gguf");

    vec![text_model_path, vision_model_path]
}

// Checks if the model files for the cpp server are downloaded
#[tauri::command]
pub fn check_model_download(app_handle: tauri::AppHandle) -> bool {
    let model_paths = get_model_paths(app_handle);

    model_paths.iter().all(|path| path.exists())
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadStarted {
    model_name: String,
    content_length: u64
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress {
    model_name: String,
    total_progress: u64
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadFinished {
    model_name: String,
}

// Downloads the model from huggingface into the data dir
#[tauri::command]
pub async fn download_model(app_handle: tauri::AppHandle) -> Result<(), String> {
    // Get the directory to save the model to
    let app_data_path = app_handle
        .path()
        .app_data_dir()
        .expect("App data dir could not be fetched.");
    let models_dir = app_data_path.join("models");
    fs::create_dir_all(&models_dir).unwrap();
    let text_model_url = "https://huggingface.co/lukesutor/Qwen2VL-2B-Q4-K-M-GGUF/resolve/main/qwen2vl-2b-text.gguf?download=true";
    let vision_model_url = "https://huggingface.co/lukesutor/Qwen2VL-2B-Q4-K-M-GGUF/resolve/main/qwen2vl-2b-vision.gguf?download=true";
    let text_model_name = "qwen2vl-2b-text.gguf";
    let vision_model_name = "qwen2vl-2b-vision.gguf";
    let text_model_path = models_dir.join(text_model_name);
    let vision_model_path = models_dir.join(vision_model_name);

    for (model_name, url, out_path) in [(text_model_name, text_model_url, text_model_path), (vision_model_name, vision_model_url, vision_model_path)] {
        let client = Client::new();
        let response = client.get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let total_size = response
            .content_length()
            .ok_or_else(|| "Failed to get content length".to_string())?;

        // Send start update
        app_handle
            .emit("download-started", DownloadStarted {
                model_name: model_name.to_string(),
                content_length: total_size
            }).unwrap();
    
        let mut file = File::create(out_path).map_err(|e| e.to_string())?;
        let mut downloaded: u64 = 0;

        let mut stream = response.bytes_stream();

        // Process the stream of chunks
        while let Some(chunk) = stream.next().await {
            let chunk_data = chunk.map_err(|e| e.to_string())?;
            file.write_all(&chunk_data).map_err(|e| e.to_string())?;
            downloaded += chunk_data.len() as u64;

            // Send progress update
            app_handle
                .emit("download-progress", DownloadProgress {
                    model_name: model_name.to_string(),
                    total_progress: downloaded
                }).unwrap();
        }

        // Send completion update
        app_handle
            .emit("download-finished", DownloadFinished {
                model_name: model_name.to_string()
            }).unwrap();
    }
    Ok(())
}

#[tauri::command]
pub fn take_screenshot(app_handle: tauri::AppHandle) -> String {
    let screens = Screen::all().unwrap();
    let screen = &screens[0]; // Assuming single screen for simplicity
    let image = screen.capture().unwrap();

    let app_data_path = app_handle
        .path()
        .app_data_dir()
        .expect("App data dir could not be fetched.");
    let screenshots_dir = app_data_path.join("screenshots");
    fs::create_dir_all(&screenshots_dir).unwrap();

    let screenshot_path = screenshots_dir.join("screenshot.png");
    image.save(screenshot_path.clone()).unwrap();
    resize_image(screenshot_path.clone());
    println!("Screenshot saved to: {:?}", screenshots_dir);
    screenshot_path.to_str().unwrap().to_string()
}

pub fn resize_image(path: PathBuf) {
    let img = image::open(&path).expect("Failed to open image");
    let resized_img = img.resize(800, 800, FilterType::Triangle);
    resized_img.save(&path).expect("Failed to save resized image");
}