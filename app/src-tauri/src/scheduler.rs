use crate::{data, prompts, vlm};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

struct SchedulerState {
    task_handle: Option<JoinHandle<()>>,
    interval_minutes: u64,
}

// Global state to hold the scheduler task handle and interval
static SCHEDULER_STATE: Lazy<Arc<Mutex<SchedulerState>>> = Lazy::new(|| {
    Arc::new(Mutex::new(SchedulerState {
        task_handle: None,
        interval_minutes: 1, // Default interval: 1 minute
    }))
});

// Define the payload for the event
#[derive(Clone, Serialize)]
struct TaskResultPayload {
    result: String,
}

// The core function that runs periodically
async fn run_scheduled_task(app_handle: tauri::AppHandle) {
    println!("[scheduler] Running scheduled task...");

    // Take screenshot
    let screenshot_path_result = data::take_screenshot(app_handle.clone());
    let screenshot_path = match screenshot_path_result {
        path => {
            println!("[scheduler] Screenshot taken: {}", path);
            path
        }
    };

    // Get prompt
    let prompt_key = "SUMMARIZE_ACTION";
    let prompt = match prompts::get_prompt(prompt_key) {
        Some(p) => {
            println!("[scheduler] Fetched prompt for key '{}'", prompt_key);
            p.to_string()
        }
        None => {
            eprintln!("[scheduler] Error: Prompt key '{}' not found.", prompt_key);
            return;
        }
    };

    // Call VLM to get response (now returns serde_json::Value)
    let vlm_response = vlm::get_vlm_response(
        app_handle.clone(),
        screenshot_path.clone(),
        prompt,
    )
    .await;

    let mut application = String::new();
    let mut description = String::new();

    match &vlm_response {
        Ok(json_val) => {
            println!("[scheduler] VLM response received successfully.");
            // Extract "application" and "description" fields from JSON
            application = json_val.get("application")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            description = json_val.get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
        }
        Err(e) => {
            eprintln!("[scheduler] Error getting VLM response: {}", e);
        }
    }

    // Embed the description text and save to a Vec<f32>
    let embedding: Vec<f32> = match &vlm_response {
        Ok(json_val) => {
            let desc = json_val.get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            match crate::embedding::get_embedding(app_handle.clone(), desc).await {
                Ok(json_val) => {
                    if let serde_json::Value::Array(arr) = json_val {
                        arr.iter()
                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                            .collect()
                    } else {
                        eprintln!("[scheduler] Embedding result is not an array.");
                        Vec::new()
                    }
                }
                Err(e) => {
                    eprintln!("[scheduler] Failed to get embedding: {}", e);
                    Vec::new()
                }
            }
        }
        Err(_) => Vec::new(),
    };

    // Insert the event into the database
    if !embedding.is_empty() && !description.is_empty() {
        let timestamp = chrono::Utc::now().timestamp();
        let active_app = application.clone();
        let description_opt = Some(description.clone());
        let description_embedding = embedding.clone();

        let db_state = match app_handle.state::<crate::db::DbState>() {
            Ok(state) => state,
            Err(e) => {
                eprintln!("[scheduler] Failed to get DB state: {}", e);
                return;
            }
        };

        match crate::db::insert_event(
            db_state,
            timestamp,
            active_app,
            description_opt,
            description_embedding,
        ) {
            Ok(_) => {
                println!("[scheduler] Event inserted into database successfully.");
            }
            Err(e) => {
                eprintln!("[scheduler] Failed to insert event into database: {}", e);
            }
        }
    } else {
        eprintln!("[scheduler] Skipping DB insert: embedding or description is empty.");
    }

    // Emit the result to the frontend (send the whole JSON if available, else error string)
    let emit_result = match &vlm_response {
        Ok(json_val) => serde_json::to_string(json_val).unwrap_or_else(|_| "".to_string()),
        Err(e) => e.to_string(),
    };

    if let Err(e) = app_handle.emit(
        "task-completed",
        TaskResultPayload {
            result: emit_result,
        },
    ) {
        eprintln!("[scheduler] Failed to emit task-completed event: {}", e);
    }
    println!("[scheduler] Scheduled task finished.");
}

#[tauri::command]
pub async fn start_scheduler(app_handle: tauri::AppHandle, interval: Option<u64>) -> Result<(), String> {
    let mut state = SCHEDULER_STATE.lock().await;

    // Stop existing task if running
    if let Some(handle) = state.task_handle.take() {
        println!("[scheduler] Aborting previous task...");
        handle.abort();
    }

    // Update interval if provided, otherwise use existing or default
    if let Some(new_interval) = interval {
        if new_interval == 0 {
            return Err("Interval must be greater than 0.".to_string());
        }
        state.interval_minutes = new_interval;
    }
    let current_interval = state.interval_minutes;
    let interval_duration = Duration::from_secs(current_interval * 60);

    println!("[scheduler] Starting scheduler with interval: {} minutes", current_interval);

    // Spawn the new task
    let handle = tokio::spawn(async move {
        loop {
            let app_handle_clone = app_handle.clone();
            run_scheduled_task(app_handle_clone).await;
            sleep(interval_duration).await;
        }
    });

    state.task_handle = Some(handle);
    println!("[scheduler] Scheduler started successfully.");
    Ok(())
}

#[tauri::command]
pub async fn stop_scheduler() -> Result<(), String> {
    let mut state = SCHEDULER_STATE.lock().await;

    if let Some(handle) = state.task_handle.take() {
        println!("[scheduler] Stopping scheduler...");
        handle.abort();
        println!("[scheduler] Scheduler stopped successfully.");
        Ok(())
    } else {
        println!("[scheduler] Scheduler is not running.");
        Err("Scheduler is not running.".to_string())
    }
}

#[tauri::command]
pub async fn get_scheduler_interval() -> Result<u64, String> {
    let state = SCHEDULER_STATE.lock().await;
    Ok(state.interval_minutes)
}
