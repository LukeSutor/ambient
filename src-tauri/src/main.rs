// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod llm;
use llm::LLMService;
use tauri::Manager;

// #[tauri::command]
// async fn generate_text(state: tauri::State<'_, LLMService>, prompt: String) -> Result<String, String> {
//     state.generate(prompt).await
// }

fn main() {
    tauri::Builder::default()
        // .setup(|app| {
        //     // Initialize LLM service with model path
        //     let llm = LLMService::new("../llm/llava-v1.6-mistral-7b.Q4_K_M.gguf".into())?;
        //     app.manager().insert(llm);
        //     Ok(())
        // })
        // .invoke_handler(tauri::generate_handler![generate_text])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}