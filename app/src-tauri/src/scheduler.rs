use crate::{data, prompts, vlm};
use once_cell::sync::Lazy;
use serde::Serialize; // Import Serialize
use std::sync::Arc;
use tauri::{Emitter, Manager}; // Import Emitter
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

    // 1. Take screenshot
    let screenshot_path_result = data::take_screenshot(app_handle.clone());
    let screenshot_path = match screenshot_path_result {
        path => {
            println!("[scheduler] Screenshot taken: {}", path);
            path
        } // Assuming take_screenshot now directly returns String or panics
          // Add proper error handling if take_screenshot returns Result
    };

    // 2. Get prompt
    let prompt_key = "SUMMARIZE_ACTION";
    let prompt = match prompts::get_prompt(prompt_key) {
        Some(p) => {
            println!("[scheduler] Fetched prompt for key '{}'", prompt_key);
            p.to_string()
        }
        None => {
            eprintln!("[scheduler] Error: Prompt key '{}' not found.", prompt_key);
            return; // Stop task execution if prompt is missing
        }
    };

    // 3. Define model and mmproj paths (consider making these configurable)
    let model = "C:/Users/Luke/Desktop/coding/local-computer-use/backend/models/smol.gguf";
    let mmproj = "C:/Users/Luke/Desktop/coding/local-computer-use/backend/models/mmproj.gguf";

    // 4. Call VLM to get response
    println!(
        "[scheduler] Getting VLM response for image: {}, model: {}, mmproj: {}",
        screenshot_path, model, mmproj
    );
    match vlm::get_vlm_response(
        app_handle.clone(),
        model.to_string(),
        mmproj.to_string(),
        screenshot_path,
        prompt,
    )
    .await
    {
        Ok(result) => {
            println!("[scheduler] VLM response received successfully.");
            // Emit the result to the frontend
            if let Err(e) = app_handle.emit(
                "task-completed",
                TaskResultPayload {
                    result: result.clone(),
                },
            ) {
                eprintln!("[scheduler] Failed to emit task-completed event: {}", e);
            }
        }
        Err(e) => {
            eprintln!("[scheduler] Error getting VLM response: {}", e);
            // Optionally emit an error event
        }
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
