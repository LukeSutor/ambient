// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
pub mod data;
pub mod vlm;
pub mod prompts;
pub mod scheduler;
pub mod embedding; // Add the new embedding module

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_fs::init())
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_opener::init())
    .invoke_handler(tauri::generate_handler![
      vlm::get_vlm_response,
      data::take_screenshot,
      prompts::get_prompt_command,
      scheduler::start_scheduler,
      scheduler::stop_scheduler,
      scheduler::get_scheduler_interval,
      embedding::get_embedding
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
