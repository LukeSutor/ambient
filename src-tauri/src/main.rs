// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod llm;
use llm::LLMService;
use tauri::Manager;
use async_std::sync::Mutex;

#[tauri::command]
async fn generate_text(state: tauri::State<'_, Mutex<LLMService>>, prompt: String) -> Result<String, String> {
    let state = state.lock().await;
    state.generate(prompt).await
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let llm_service = LLMService::new("llm/models/llava-v1.6-mistral-7b.Q4_K_M.gguf".into())
                .map_err(|e| e.to_string())?;
            app.manage(Mutex::new(llm_service));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![generate_text])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}