//! State management for the browser-use runtime.
//!
//! Provides a global state for managing browser-use session lifecycle,
//! including cancellation signals and active session tracking.
//! Uses `AtomicBool` for synchronous checks and `tokio::sync::Notify`
//! for instant async cancellation.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::Notify;

/// Global state for the browser-use runtime.
///
/// Manages cancellation signals and active session tracking.
pub struct BrowserUseState {
    /// Flag to signal that the current session should stop.
    pub should_stop: Arc<AtomicBool>,
    /// Async notification for instant cancellation of blocked I/O.
    cancel_notify: std::sync::Mutex<Arc<Notify>>,
    /// Whether a session is currently running.
    pub is_running: Mutex<bool>,
    /// The conversation ID of the currently running session (if any).
    pub active_conversation_id: Mutex<Option<String>>,
}

impl Default for BrowserUseState {
    fn default() -> Self {
        Self {
            should_stop: Arc::new(AtomicBool::new(false)),
            cancel_notify: std::sync::Mutex::new(Arc::new(Notify::new())),
            is_running: Mutex::new(false),
            active_conversation_id: Mutex::new(None),
        }
    }
}

impl BrowserUseState {
    /// Signal that the session should stop.
    pub fn signal_stop(&self) {
        self.should_stop.store(true, Ordering::SeqCst);
        self.cancel_notify.lock().unwrap_or_else(|e| e.into_inner()).notify_waiters();
    }

    /// Reset the stop signal (called when starting a new session).
    fn reset_stop_signal(&self) {
        self.should_stop.store(false, Ordering::SeqCst);
        *self.cancel_notify.lock().unwrap_or_else(|e| e.into_inner()) = Arc::new(Notify::new());
    }

    /// Mark session as started.
    pub async fn start_session(&self, conversation_id: &str) -> Result<(), String> {
        let mut is_running = self.is_running.lock().await;
        if *is_running {
            return Err("A browser-use session is already running".to_string());
        }
        *is_running = true;

        let mut active_conv = self.active_conversation_id.lock().await;
        *active_conv = Some(conversation_id.to_string());

        self.reset_stop_signal();
        Ok(())
    }

    /// Mark session as finished.
    pub async fn finish_session(&self) {
        let mut is_running = self.is_running.lock().await;
        *is_running = false;

        let mut active_conv = self.active_conversation_id.lock().await;
        *active_conv = None;
    }

    /// Get the stop signal as an Arc for sharing with async tasks.
    pub fn get_stop_signal(&self) -> Arc<AtomicBool> {
        self.should_stop.clone()
    }

    /// Get the cancel notify for sharing with the LLM client.
    pub fn get_cancel_notify(&self) -> Arc<Notify> {
        self.cancel_notify.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

/// Stop the current browser-use session.
#[tauri::command]
pub async fn stop_browser_use(
    state: tauri::State<'_, BrowserUseState>,
) -> Result<String, String> {
    log::info!("[browser_use] Stop signal requested");
    state.signal_stop();
    Ok("Stop signal sent".to_string())
}
