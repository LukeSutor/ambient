//! Notification system for automations.
//!
//! Sends notifications to the HUD overlay when automation tasks
//! complete or encounter errors.

use super::types::*;
use tauri::AppHandle;

/// Send a completion notification to the HUD.
pub fn send_completion_notification(
    _app_handle: &AppHandle,
    task: &AutomationTask,
    result_text: &str,
) {
    let notification = AutomationNotification {
        id: uuid::Uuid::new_v4().to_string(),
        task_id: task.id.clone(),
        task_name: task.name.clone(),
        notification_type: NotificationType::Success,
        title: format!("{} completed", task.name),
        body: truncate_text(result_text, 200),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let _ = crate::events::emitter::emit(
        super::events::AUTOMATION_NOTIFICATION,
        notification,
    );
}

/// Send an error notification to the HUD.
pub fn send_error_notification(
    _app_handle: &AppHandle,
    task: &AutomationTask,
    error: &str,
) {
    let notification = AutomationNotification {
        id: uuid::Uuid::new_v4().to_string(),
        task_id: task.id.clone(),
        task_name: task.name.clone(),
        notification_type: NotificationType::Error,
        title: format!("{} failed", task.name),
        body: truncate_text(error, 200),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let _ = crate::events::emitter::emit(
        super::events::AUTOMATION_NOTIFICATION,
        notification,
    );
}

/// Truncate text to a maximum length, appending "..." if needed.
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len])
    }
}
