// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod control;
pub mod sidecar;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            sidecar::start_server,
            sidecar::shutdown_server,
            sidecar::infer,
            control::move_mouse,
            control::click_mouse,
            control::type_string
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
