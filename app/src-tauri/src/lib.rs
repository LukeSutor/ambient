// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
pub mod data;
pub mod sidecar;
pub mod prompts;
pub mod scheduler; // Add the new scheduler module

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_fs::init())
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_opener::init())
    .invoke_handler(tauri::generate_handler![
      sidecar::call_main_sidecar,
      sidecar::call_llama_sidecar,
      data::take_screenshot,
      prompts::get_prompt_command,
      scheduler::start_scheduler, // Add start command
      scheduler::stop_scheduler,  // Add stop command
      scheduler::get_scheduler_interval // Add command to get interval
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
