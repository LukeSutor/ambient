use crate::events::{emitter, types::*};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};
use tokio_util::sync::CancellationToken;
use chrono;

struct SchedulerState {
  task_handle: Option<JoinHandle<()>>,
  capture_handle: Option<JoinHandle<()>>,
  capture_cancel_token: Option<CancellationToken>,
  interval_minutes: u64,
  capture_enabled: bool,
}

// Global state to hold the scheduler task handle and interval
static SCHEDULER_STATE: Lazy<Arc<Mutex<SchedulerState>>> = Lazy::new(|| {
  Arc::new(Mutex::new(SchedulerState {
    task_handle: None,
    capture_handle: None,
    capture_cancel_token: None,
    interval_minutes: 1,
    capture_enabled: false,
  }))
});

// Define the payload for the event
#[derive(Clone, Serialize)]
struct TaskResultPayload {
  result: String,
}

// Function to emit CAPTURE_SCREEN event every 10 seconds
async fn run_capture_screen_task(cancel_token: CancellationToken) {
  let mut interval = interval(Duration::from_secs(10));
  
  loop {
    tokio::select! {
      _ = interval.tick() => {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let capture_event = CaptureScreenEvent { timestamp };
        
        if let Err(e) = emitter::emit(CAPTURE_SCREEN, capture_event) {
          eprintln!("[scheduler] Failed to emit CAPTURE_SCREEN event: {}", e);
        } else {
          println!("[scheduler] Emitted CAPTURE_SCREEN event");
        }
      }
      _ = cancel_token.cancelled() => {
        println!("[scheduler] Capture task cancelled gracefully");
        break;
      }
    }
  }
}

#[tauri::command]
pub async fn get_scheduler_interval() -> Result<u64, String> {
  let state = SCHEDULER_STATE.lock().await;
  Ok(state.interval_minutes)
}

#[tauri::command]
pub async fn start_capture_scheduler() -> Result<(), String> {
  let mut state = SCHEDULER_STATE.lock().await;

  // Stop existing capture task if running
  if let Some(cancel_token) = state.capture_cancel_token.take() {
    println!("[scheduler] Cancelling previous capture task...");
    cancel_token.cancel();
  }
  
  if let Some(handle) = state.capture_handle.take() {
    // Wait a moment for graceful shutdown, then abort if needed
    tokio::time::timeout(Duration::from_millis(100), async {
      let _ = handle.await;
    }).await.ok();
  }

  println!("[scheduler] Starting capture screen scheduler (10 second interval)");

  // Create new cancellation token
  let cancel_token = CancellationToken::new();
  let cancel_token_clone = cancel_token.clone();

  // Spawn the new capture task
  let handle = tokio::spawn(async move {
    run_capture_screen_task(cancel_token_clone).await;
  });

  state.capture_handle = Some(handle);
  state.capture_cancel_token = Some(cancel_token);
  state.capture_enabled = true;
  println!("[scheduler] Capture scheduler started successfully.");
  Ok(())
}

#[tauri::command]
pub async fn stop_capture_scheduler() -> Result<(), String> {
  let mut state = SCHEDULER_STATE.lock().await;

  if let Some(cancel_token) = state.capture_cancel_token.take() {
    println!("[scheduler] Stopping capture scheduler...");
    
    // Request graceful cancellation
    cancel_token.cancel();
    
    // Wait for the task to finish gracefully
    if let Some(handle) = state.capture_handle.take() {
      match tokio::time::timeout(Duration::from_secs(1), handle).await {
        Ok(_) => println!("[scheduler] Capture scheduler stopped gracefully."),
        Err(_) => {
          println!("[scheduler] Capture scheduler timed out, but should stop soon.");
        }
      }
    }
    
    state.capture_enabled = false;
    Ok(())
  } else {
    println!("[scheduler] Capture scheduler is not running.");
    Err("Capture scheduler is not running.".to_string())
  }
}

#[tauri::command]
pub async fn is_scheduler_running() -> Result<bool, String> {
  let state = SCHEDULER_STATE.lock().await;
  Ok(state.capture_enabled)
}
