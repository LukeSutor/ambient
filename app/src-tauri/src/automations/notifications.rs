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
        body: result_text.to_string(),
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
        body: error.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let _ = crate::events::emitter::emit(
        super::events::AUTOMATION_NOTIFICATION,
        notification,
    );
}
