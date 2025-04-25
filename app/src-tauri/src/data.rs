use image::imageops::FilterType;
use screenshots::Screen;
use tauri::{path::BaseDirectory, Manager};
use std::{fs::File, io::Write};
use std::path::PathBuf;
use std::fs;

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
  resized_img
    .save(&path)
    .expect("Failed to save resized image");
}
