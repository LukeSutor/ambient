//! Agent runtime state management for cancellation support.
//!
//! This module provides a global state for managing agent runtime cancellation.
//! Uses `AtomicBool` for synchronous checks (between loop iterations) and
//! `tokio::sync::Notify` for instant async cancellation (interrupts blocked I/O).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::Notify;

/// Global state for the agent runtime.
/// 
/// Manages cancellation signals and active request tracking.
/// The dual-signal design ensures both fast async cancellation
/// (via `Notify`, which drops in-flight HTTP requests) and
/// reliable synchronous polling (via `AtomicBool`).
pub struct AgentRuntimeState {
    /// Flag to signal that the current generation should stop.
    pub should_stop: Arc<AtomicBool>,
    /// Async notification for instant cancellation of blocked I/O.
    /// Replaced with a fresh instance on each new generation to avoid stale permits.
    cancel_notify: std::sync::Mutex<Arc<Notify>>,
    /// Whether a generation is currently in progress.
    pub is_running: Mutex<bool>,
    /// The conversation ID of the currently running generation (if any).
    pub active_conversation_id: Mutex<Option<String>>,
}

impl Default for AgentRuntimeState {
    fn default() -> Self {
        Self {
            should_stop: Arc::new(AtomicBool::new(false)),
            cancel_notify: std::sync::Mutex::new(Arc::new(Notify::new())),
            is_running: Mutex::new(false),
            active_conversation_id: Mutex::new(None),
        }
    }
}

impl AgentRuntimeState {
    /// Check if the generation should stop.
    pub fn should_stop(&self) -> bool {
        self.should_stop.load(Ordering::SeqCst)
    }

    /// Signal that the generation should stop.
    ///
    /// Sets the atomic flag AND sends an async notification, which
    /// immediately wakes any `tokio::select!` racing against the
    /// cancel signal — even if the provider is blocked waiting for
    /// the next chunk from the LLM server.
    pub fn signal_stop(&self) {
        self.should_stop.store(true, Ordering::SeqCst);
        self.cancel_notify.lock().unwrap().notify_waiters();
    }

    /// Reset the stop signal (called when starting a new generation).
    fn reset_stop_signal(&self) {
        self.should_stop.store(false, Ordering::SeqCst);
        // Create a fresh Notify to discard any stale permits from
        // the previous generation's cancellation.
        *self.cancel_notify.lock().unwrap() = Arc::new(Notify::new());
    }

    /// Mark generation as started.
    pub async fn start_generation(&self, conversation_id: &str) -> Result<(), String> {
        let mut is_running = self.is_running.lock().await;
        if *is_running {
            return Err("A generation is already in progress".to_string());
        }
        *is_running = true;
        
        let mut active_conv = self.active_conversation_id.lock().await;
        *active_conv = Some(conversation_id.to_string());
        
        self.reset_stop_signal();
        Ok(())
    }

    /// Mark generation as finished.
    pub async fn finish_generation(&self) {
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
        self.cancel_notify.lock().unwrap().clone()
    }
}

/// Stop the current agent generation.
#[tauri::command]
pub async fn stop_agent_chat(
    state: tauri::State<'_, AgentRuntimeState>,
) -> Result<String, String> {
    log::info!("[agent] Stop signal requested");
    state.signal_stop();
    Ok("Stop signal sent".to_string())
}
