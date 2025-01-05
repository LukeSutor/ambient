// Contains functions for the application data control

use hf_hub::api::{sync::Api, Progress};
use screenshots::Screen;
use std::fs;
use tauri::{Wry, Manager};
use tauri_plugin_store::Store;

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

#[tauri::command]
pub fn check_model_download() -> bool {
    // Checks if the model files for the cpp server are downloaded
    true
}

#[tauri::command]
pub fn download_model() {
    // Downloads the model from huggingface into the cache dir
    let api = Api::new().unwrap();
    let text_model_progress = MyProgress {
        current: 0,
        total: 0,
    };
    let text_model = api
        .model("lukesutor/Qwen2VL-2B-Q4-K-M-GGUF".to_string())
        .download_with_progress("qwen2vl-2b-text.gguf", text_model_progress)
        .unwrap();
    println!("{}", text_model.to_str().unwrap().to_string());
    let vision_model_progress = MyProgress {
        current: 0,
        total: 0,
    };
    let vision_model = api
        .model("lukesutor/Qwen2VL-2B-Q4-K-M-GGUF".to_string())
        .download_with_progress("qwen2vl-2b-vision.gguf", vision_model_progress)
        .unwrap();
    println!("{}", vision_model.to_str().unwrap().to_string());
}

#[tauri::command]
pub fn take_screenshot(app_handle: tauri::AppHandle) -> String {
    let screens = Screen::all().unwrap();
    let screen = &screens[0]; // Assuming single screen for simplicity
    let image = screen.capture().unwrap();

    // let store = Store::get()
    let app_data_path = app_handle
        .path()
        .app_data_dir()
        .expect("App data dir could not be fetched.");
    let screenshots_dir = app_data_path.join("screenshots");
    fs::create_dir_all(&screenshots_dir).unwrap();

    let screenshot_path = screenshots_dir.join("screenshot.png");
    image.save(screenshot_path.clone()).unwrap();
    println!("Screenshot saved to: {:?}", screenshots_dir);
    screenshot_path.to_str().unwrap().to_string()
}
