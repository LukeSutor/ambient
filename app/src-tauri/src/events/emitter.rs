use tauri::{AppHandle, Manager};
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use serde::Serialize;

pub struct EventEmitter {
    app_handle: Arc<Mutex<Option<AppHandle>>>,
}

impl EventEmitter {
    fn new() -> Self {
        Self {
            app_handle: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_app_handle(&self, handle: AppHandle) {
        let mut app_handle = self.app_handle.lock().unwrap();
        *app_handle = Some(handle);
    }

    pub fn emit<T: Serialize + Clone>(&self, event: &str, payload: T) -> Result<(), String> {
        let app_handle = self.app_handle.lock().unwrap();
        if let Some(handle) = app_handle.as_ref() {
            handle.emit_all(event, payload)
                .map_err(|e| format!("Failed to emit event '{}': {}", event, e))
        } else {
            Err("AppHandle not initialized".to_string())
        }
    }

    pub fn emit_to_window<T: Serialize + Clone>(&self, window_label: &str, event: &str, payload: T) -> Result<(), String> {
        let app_handle = self.app_handle.lock().unwrap();
        if let Some(handle) = app_handle.as_ref() {
            if let Some(window) = handle.get_window(window_label) {
                window.emit(event, payload)
                    .map_err(|e| format!("Failed to emit event '{}' to window '{}': {}", event, window_label, e))
            } else {
                Err(format!("Window '{}' not found", window_label))
            }
        } else {
            Err("AppHandle not initialized".to_string())
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.app_handle.lock().unwrap().is_some()
    }
}

// Global singleton instance
static EVENT_EMITTER: Lazy<EventEmitter> = Lazy::new(|| EventEmitter::new());

// Public functions to access the singleton
pub fn get_emitter() -> &'static EventEmitter {
    &EVENT_EMITTER
}

pub fn emit<T: Serialize + Clone>(event: &str, payload: T) -> Result<(), String> {
    get_emitter().emit(event, payload)
}

pub fn emit_to_window<T: Serialize + Clone>(window_label: &str, event: &str, payload: T) -> Result<(), String> {
    get_emitter().emit_to_window(window_label, event, payload)
}