// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// use std::sync::{Arc, Mutex};
// use tauri::{Emitter, Manager, RunEvent};
// use tauri_plugin_shell::process::{CommandChild, CommandEvent};
// use tauri_plugin_shell::ShellExt;

// // Helper function to spawn the sidecar and monitor its stdout/stderr
// fn spawn_and_monitor_sidecar(app_handle: tauri::AppHandle) -> Result<(), String> {
//     // Check if a sidecar process already exists
//     if let Some(state) = app_handle.try_state::<Arc<Mutex<Option<CommandChild>>>>() {
//         let child_process = state.lock().unwrap();
//         if child_process.is_some() {
//             // A sidecar is already running, do not spawn a new one
//             println!("[tauri] Sidecar is already running. Skipping spawn.");
//             return Ok(()); // Exit early since sidecar is already running
//         }
//     }
//     // Spawn sidecar
//     let sidecar_command = app_handle
//         .shell()
//         .sidecar("main")
//         .map_err(|e| e.to_string())?;
//     let (mut rx, child) = sidecar_command.spawn().map_err(|e| e.to_string())?;
//     // Store the child process in the app state
//     if let Some(state) = app_handle.try_state::<Arc<Mutex<Option<CommandChild>>>>() {
//         *state.lock().unwrap() = Some(child);
//     } else {
//         return Err("Failed to access app state".to_string());
//     }

//     // Spawn an async task to handle sidecar communication
//     tauri::async_runtime::spawn(async move {
//         while let Some(event) = rx.recv().await {
//             match event {
//                 CommandEvent::Stdout(line_bytes) => {
//                     let line = String::from_utf8_lossy(&line_bytes);
//                     println!("Sidecar stdout: {}", line);
//                     // Emit the line to the frontend
//                     app_handle
//                         .emit("sidecar-stdout", line.to_string())
//                         .expect("Failed to emit sidecar stdout event");
//                 }
//                 CommandEvent::Stderr(line_bytes) => {
//                     let line = String::from_utf8_lossy(&line_bytes);
//                     eprintln!("Sidecar stderr: {}", line);
//                     // Emit the error line to the frontend
//                     app_handle
//                         .emit("sidecar-stderr", line.to_string())
//                         .expect("Failed to emit sidecar stderr event");
//                 }
//                 _ => {}
//             }
//         }
//     });

//     Ok(())
// }

fn main() {
    tauri::Builder::default()
        // .setup(|app| {
        //     // Initialize app state for sidecar process
        //     app.manage(Arc::new(Mutex::new(None::<CommandChild>)));
        //     let app_handle = app.handle().clone();
        //     // Spawn the sidecar process
        //     spawn_and_monitor_sidecar(app_handle).ok();
        //     Ok(())
        // })
        // .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}