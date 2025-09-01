use image::imageops::{FilterType, crop};
use screenshots::Screen;
use std::fs;
use std::path::PathBuf;
use tauri::Manager;
use crate::screen_selection::SelectionBounds;

#[tauri::command]
pub fn take_screenshot(app_handle: tauri::AppHandle, filename: String) -> String {
  let screens = Screen::all().unwrap();
  let screen = &screens[0]; // Assuming single screen for simplicity
  let image = screen.capture().unwrap();

  let app_data_path = app_handle
    .path()
    .app_data_dir()
    .expect("App data dir could not be fetched.");
  let screenshots_dir = app_data_path.join("screenshots");
  fs::create_dir_all(&screenshots_dir).unwrap();

  let screenshot_path = screenshots_dir.join(filename);
  image.save(screenshot_path.clone()).unwrap();
  log::info!("Screenshot saved to: {:?}", screenshots_dir);
  screenshot_path.to_str().unwrap().to_string()
}

pub fn crop_image_selection(path: PathBuf, selection: SelectionBounds) {
  let mut img = image::open(&path).expect("Failed to open image");
  let cropped_img = crop(&mut img, selection.x.try_into().unwrap(), selection.y.try_into().unwrap(), selection.width.try_into().unwrap(), selection.height.try_into().unwrap());
  cropped_img
    .to_image()
    .save(&path)
    .expect("Failed to save cropped image");
}

pub fn resize_image(path: PathBuf) {
  let img = image::open(&path).expect("Failed to open image");
  let resized_img = img.resize(800, 800, FilterType::Triangle);
  resized_img
    .save(&path)
    .expect("Failed to save resized image");
}
