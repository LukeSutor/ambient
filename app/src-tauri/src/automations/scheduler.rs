//! Time-based scheduling engine for automations.
//!
//! Uses tokio timers to schedule tasks at intervals or specific times.
//! Manages a global map of running scheduled tasks with cancellation support.

use super::types::AutomationTask;
use chrono::Datelike;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;
use tokio::sync::RwLock;

/// Handle to a running scheduled task, allowing cancellation.
struct ScheduledTaskHandle {
    cancel: Arc<AtomicBool>,
}

/// Global map of task_id → handle for all active scheduled tasks.
static SCHEDULER: Lazy<RwLock<HashMap<String, ScheduledTaskHandle>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Schedule a task to run on its configured schedule.
///
/// Spawns a tokio task that loops: sleep until next run → execute → repeat.
/// For `once` tasks, the loop runs exactly once.
pub async fn schedule_task(
    app_handle: &AppHandle,
    task: &AutomationTask,
) -> Result<(), String> {
    if task.task_type != "scheduled" {
        return Err("Only scheduled tasks can be registered with the scheduler".to_string());
    }

    let schedule_type = task.schedule_type.as_deref().unwrap_or("interval").to_string();
    let schedule_value = task.schedule_value.as_deref().unwrap_or("15").to_string();

    // Validate and parse the schedule
    let _ = parse_schedule_duration(&schedule_type, &schedule_value)?;

    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_clone = cancel.clone();
    let task_id = task.id.clone();
    let task_clone = task.clone();
    let app = app_handle.clone();

    // Store the handle first
    {
        let mut scheduler = SCHEDULER.write().await;
        // Cancel any existing schedule for this task
        if let Some(existing) = scheduler.remove(&task_id) {
            existing.cancel.store(true, Ordering::SeqCst);
        }
        scheduler.insert(
            task_id.clone(),
            ScheduledTaskHandle { cancel: cancel_clone },
        );
    }

    // Spawn the scheduling loop
    tokio::spawn(async move {
        log::info!(
            "[scheduler] Task '{}' ({}) scheduled: {} = {}",
            task_clone.name,
            task_clone.id,
            schedule_type,
            schedule_value
        );

        loop {
            if cancel.load(Ordering::SeqCst) {
                log::info!("[scheduler] Task '{}' cancelled", task_clone.id);
                break;
            }

            // Calculate duration until next run
            let duration = match calculate_next_duration(
                task_clone.schedule_type.as_deref().unwrap_or("interval"),
                task_clone.schedule_value.as_deref().unwrap_or("15"),
            ) {
                Ok(d) => d,
                Err(e) => {
                    log::error!(
                        "[scheduler] Failed to calculate next run for '{}': {}",
                        task_clone.id,
                        e
                    );
                    break;
                }
            };

            log::info!(
                "[scheduler] Task '{}' sleeping for {:?}",
                task_clone.id,
                duration
            );

            // Sleep, checking for cancellation periodically
            let sleep_start = tokio::time::Instant::now();
            let target = sleep_start + duration;

            loop {
                if cancel.load(Ordering::SeqCst) {
                    log::info!("[scheduler] Task '{}' cancelled during sleep", task_clone.id);
                    // Remove from map before exiting
                    let mut scheduler = SCHEDULER.write().await;
                    scheduler.remove(&task_clone.id);
                    return;
                }

                let remaining = target.saturating_duration_since(tokio::time::Instant::now());
                if remaining.is_zero() {
                    break;
                }

                // Check cancellation every 5 seconds or remaining time, whichever is smaller
                let check_interval = std::cmp::min(
                    remaining,
                    tokio::time::Duration::from_secs(5),
                );
                tokio::time::sleep(check_interval).await;
            }

            if cancel.load(Ordering::SeqCst) {
                break;
            }

            // Execute the automation
            log::info!("[scheduler] Executing scheduled task '{}'", task_clone.id);

            // Re-read the task from DB to get latest state
            match super::db::get_task_by_id(&app, &task_clone.id) {
                Ok(current_task) => {
                    if !current_task.is_enabled {
                        log::info!(
                            "[scheduler] Task '{}' is disabled, skipping execution",
                            task_clone.id
                        );
                        break;
                    }

                    match super::executor::execute_automation(&app, &current_task).await {
                        Ok(run) => {
                            log::info!(
                                "[scheduler] Task '{}' completed with status: {}",
                                task_clone.id,
                                run.status
                            );
                        }
                        Err(e) => {
                            log::error!(
                                "[scheduler] Task '{}' execution failed: {}",
                                task_clone.id,
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    log::error!(
                        "[scheduler] Could not re-read task '{}': {}. Stopping schedule.",
                        task_clone.id,
                        e
                    );
                    break;
                }
            }

            // For `once` tasks, only execute once
            if task_clone.schedule_type.as_deref() == Some("once") {
                log::info!(
                    "[scheduler] One-time task '{}' finished, removing schedule",
                    task_clone.id
                );
                break;
            }
        }

        // Clean up
        let mut scheduler = SCHEDULER.write().await;
        scheduler.remove(&task_clone.id);
    });

    Ok(())
}

/// Remove a task from the scheduler.
pub async fn unschedule_task(task_id: &str) {
    let mut scheduler = SCHEDULER.write().await;
    if let Some(handle) = scheduler.remove(task_id) {
        handle.cancel.store(true, Ordering::SeqCst);
        log::info!("[scheduler] Unscheduled task '{}'", task_id);
    }
}

/// Reschedule all enabled scheduled tasks on app startup.
///
/// Called after database is initialized to restore all active schedules.
pub async fn reschedule_all(app_handle: &AppHandle) {
    match super::db::get_enabled_scheduled_tasks(app_handle) {
        Ok(tasks) => {
            log::info!(
                "[scheduler] Rescheduling {} enabled scheduled tasks",
                tasks.len()
            );
            for task in &tasks {
                if let Err(e) = schedule_task(app_handle, task).await {
                    log::warn!(
                        "[scheduler] Failed to reschedule task '{}': {}",
                        task.id,
                        e
                    );
                }
            }
        }
        Err(e) => {
            log::warn!("[scheduler] Failed to load scheduled tasks: {}", e);
        }
    }
}

/// Parse a schedule into a one-shot duration.
fn parse_schedule_duration(schedule_type: &str, schedule_value: &str) -> Result<std::time::Duration, String> {
    match schedule_type {
        "interval" => {
            let minutes: u64 = schedule_value
                .parse()
                .map_err(|_| format!("Invalid interval value: {}", schedule_value))?;
            if minutes == 0 {
                return Err("Interval must be at least 1 minute".to_string());
            }
            Ok(std::time::Duration::from_secs(minutes * 60))
        }
        "daily" | "weekly" | "once" => {
            // These are calculated dynamically in calculate_next_duration
            Ok(std::time::Duration::from_secs(60)) // placeholder
        }
        _ => Err(format!("Unknown schedule type: {}", schedule_type)),
    }
}

/// Calculate the duration until the next run based on schedule type and value.
fn calculate_next_duration(
    schedule_type: &str,
    schedule_value: &str,
) -> Result<std::time::Duration, String> {
    match schedule_type {
        "interval" => {
            let minutes: u64 = schedule_value
                .parse()
                .map_err(|_| format!("Invalid interval value: {}", schedule_value))?;
            Ok(std::time::Duration::from_secs(minutes * 60))
        }
        "daily" => {
            // schedule_value = "HH:MM" (e.g. "17:00")
            let parts: Vec<&str> = schedule_value.split(':').collect();
            if parts.len() != 2 {
                return Err(format!("Invalid daily schedule value: {}. Expected HH:MM", schedule_value));
            }
            let hour: u32 = parts[0].parse().map_err(|_| "Invalid hour")?;
            let minute: u32 = parts[1].parse().map_err(|_| "Invalid minute")?;

            let now = chrono::Local::now();
            let today_target = now
                .date_naive()
                .and_hms_opt(hour, minute, 0)
                .ok_or("Invalid time")?;
            let today_target = chrono::Local
                .from_local_datetime(&today_target)
                .single()
                .ok_or("Ambiguous local time")?;

            let target = if today_target > now {
                today_target
            } else {
                // Schedule for tomorrow
                today_target + chrono::Duration::days(1)
            };

            let duration = (target - now).to_std().map_err(|e| format!("Duration error: {}", e))?;
            Ok(duration)
        }
        "weekly" => {
            // schedule_value = "DAY,HH:MM" (e.g. "monday,09:00")
            let parts: Vec<&str> = schedule_value.split(',').collect();
            if parts.len() != 2 {
                return Err(format!(
                    "Invalid weekly schedule value: {}. Expected DAY,HH:MM",
                    schedule_value
                ));
            }
            let day_str = parts[0].trim().to_lowercase();
            let time_parts: Vec<&str> = parts[1].trim().split(':').collect();
            if time_parts.len() != 2 {
                return Err("Invalid time format in weekly schedule".to_string());
            }
            let hour: u32 = time_parts[0].parse().map_err(|_| "Invalid hour")?;
            let minute: u32 = time_parts[1].parse().map_err(|_| "Invalid minute")?;

            let target_weekday = match day_str.as_str() {
                "monday" | "mon" => chrono::Weekday::Mon,
                "tuesday" | "tue" => chrono::Weekday::Tue,
                "wednesday" | "wed" => chrono::Weekday::Wed,
                "thursday" | "thu" => chrono::Weekday::Thu,
                "friday" | "fri" => chrono::Weekday::Fri,
                "saturday" | "sat" => chrono::Weekday::Sat,
                "sunday" | "sun" => chrono::Weekday::Sun,
                _ => return Err(format!("Invalid weekday: {}", day_str)),
            };

            let now = chrono::Local::now();
            let current_weekday = now.weekday();
            let days_ahead = (target_weekday.num_days_from_monday() as i64
                - current_weekday.num_days_from_monday() as i64
                + 7) % 7;

            let target_date = now.date_naive() + chrono::Duration::days(days_ahead);
            let target_dt = target_date
                .and_hms_opt(hour, minute, 0)
                .ok_or("Invalid time")?;
            let target_dt = chrono::Local
                .from_local_datetime(&target_dt)
                .single()
                .ok_or("Ambiguous local time")?;

            let target = if target_dt > now {
                target_dt
            } else {
                // It's the same weekday but the time already passed → next week
                target_dt + chrono::Duration::weeks(1)
            };

            let duration = (target - now).to_std().map_err(|e| format!("Duration error: {}", e))?;
            Ok(duration)
        }
        "once" => {
            // schedule_value = ISO 8601 datetime
            let target = chrono::DateTime::parse_from_rfc3339(schedule_value)
                .map_err(|e| format!("Invalid once schedule value: {}. Expected ISO 8601: {}", schedule_value, e))?;
            let now = chrono::Utc::now();
            let duration = target
                .signed_duration_since(now)
                .to_std()
                .unwrap_or(std::time::Duration::from_secs(0));
            Ok(duration)
        }
        _ => Err(format!("Unknown schedule type: {}", schedule_type)),
    }
}

use chrono::TimeZone;
