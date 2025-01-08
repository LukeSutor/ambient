// Contains functions for the application data control

use hf_hub::api::{sync::Api, Progress};
use screenshots::Screen;
use std::fs;
use tauri::Manager;
use image::imageops::FilterType;
use std::path::PathBuf;

struct MyProgress {
    current: usize,
    total: usize,
}

impl Progress for MyProgress {
    fn init(&mut self, size: usize, _filename: &str) {
        self.total = size;
        self.current = 0;
    }

    fn update(&mut self, size: usize) {
        self.current += size;
        println!("{}/{}", self.current, self.total)
    }

    fn finish(&mut self) {
        println!("Done !");
    }
}

// Checks if the model files for the cpp server are downloaded
#[tauri::command]
pub fn check_model_download(app_handle: tauri::AppHandle) -> bool {
    // Get the directory to save the model to
    let app_data_path = app_handle
        .path()
        .app_data_dir()
        .expect("App data dir could not be fetched.");
    let models_dir = app_data_path.join("models");

    if !models_dir.exists() {
        return false;
    }

    let text_model_path = models_dir.join("qwen2vl-2b-text.gguf");
    let vision_model_path = models_dir.join("qwen2vl-2b-vision.gguf");

    text_model_path.exists() && vision_model_path.exists()
}

// Downloads the model from huggingface into the cache dir
#[tauri::command]
pub fn download_model(app_handle: tauri::AppHandle) {
    // Get the directory to save the model to
    let app_data_path = app_handle
        .path()
        .app_data_dir()
        .expect("App data dir could not be fetched.");
    let models_dir = app_data_path.join("models");
    fs::create_dir_all(&models_dir).unwrap();

    let api = Api::new().unwrap();

    // Download text model
    let text_model_progress = MyProgress {
        current: 0,
        total: 0,
    };
    let text_model_path = models_dir.join("qwen2vl-2b-text.gguf");
    let text_model = api
        .model("lukesutor/Qwen2VL-2B-Q4-K-M-GGUF".to_string())
        .download_with_progress(text_model_path.to_str().unwrap(), text_model_progress)
        .unwrap();
    println!("[tauri] Downloaded text model to {}", text_model.to_str().unwrap().to_string());

    // Vision model progress
    let vision_model_progress = MyProgress {
        current: 0,
        total: 0,
    };
    let vision_model_path = models_dir.join("qwen2vl-2b-vision.gguf");
    let vision_model = api
        .model("lukesutor/Qwen2VL-2B-Q4-K-M-GGUF".to_string())
        .download_with_progress(vision_model_path.to_str().unwrap(), vision_model_progress)
        .unwrap();
    println!("[tauri] Downloaded vision model to {}", vision_model.to_str().unwrap().to_string());
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