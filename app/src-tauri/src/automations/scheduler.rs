//! Time-based scheduling engine for automations.
//!
//! Uses tokio timers to schedule tasks at intervals or specific times.
//! Manages a global map of running scheduled tasks with cancellation support.
//! All time-based schedules operate in the user's local timezone.

use super::types::AutomationTask;
use chrono::Datelike;
use chrono::TimeZone;
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
/// If a task with the same ID is already scheduled, it is cancelled first.
pub async fn schedule_task(
    app_handle: &AppHandle,
    task: &AutomationTask,
) -> Result<(), String> {
    if task.task_type != "scheduled" {
        return Err("Only scheduled tasks can be registered with the scheduler".to_string());
    }

    let schedule_type = task.schedule_type.as_deref().unwrap_or("interval").to_string();
    let schedule_value = task.schedule_value.as_deref().unwrap_or("15").to_string();

    // Validate the schedule
    let _ = calculate_next_duration(&schedule_type, &schedule_value)?;

    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_for_handle = cancel.clone();
    let task_id = task.id.clone();
    let task_clone = task.clone();
    let app = app_handle.clone();

    // Store the handle, cancelling any existing schedule for this task
    {
        let mut scheduler = SCHEDULER.write().await;
        if let Some(existing) = scheduler.remove(&task_id) {
            existing.cancel.store(true, Ordering::SeqCst);
        }
        scheduler.insert(
            task_id.clone(),
            ScheduledTaskHandle { cancel: cancel_for_handle },
        );
    }

    // Compute and store next_run_at
    if let Ok(next) = calculate_next_run_time(&schedule_type, &schedule_value) {
        let _ = super::db::update_task_run_times(&app, &task_id, None, Some(&next));
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
            let duration = match calculate_next_duration(&schedule_type, &schedule_value) {
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
                    // Only remove from map if we're still the current schedule
                    let mut scheduler = SCHEDULER.write().await;
                    if let Some(handle) = scheduler.get(&task_clone.id) {
                        if Arc::ptr_eq(&cancel, &handle.cancel) {
                            scheduler.remove(&task_clone.id);
                        }
                    }
                    return;
                }

                let remaining = target.saturating_duration_since(tokio::time::Instant::now());
                if remaining.is_zero() {
                    break;
                }

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

            // After execution, compute and store next_run_at
            if let Ok(next) = calculate_next_run_time(&schedule_type, &schedule_value) {
                let _ = super::db::update_task_run_times(
                    &app,
                    &task_clone.id,
                    None,
                    Some(&next),
                );
            }
        }

        // Clean up — only remove from map if we're still the current schedule.
        // This prevents a superseded task from removing a newly-scheduled entry.
        let mut scheduler = SCHEDULER.write().await;
        if let Some(handle) = scheduler.get(&task_clone.id) {
            if Arc::ptr_eq(&cancel, &handle.cancel) {
                scheduler.remove(&task_clone.id);
            }
        }
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

/// Calculate the next run time as an RFC3339 string in the local timezone.
pub fn calculate_next_run_time(
    schedule_type: &str,
    schedule_value: &str,
) -> Result<String, String> {
    let duration = calculate_next_duration(schedule_type, schedule_value)?;
    let next = chrono::Local::now()
        + chrono::Duration::from_std(duration)
            .map_err(|e| format!("Duration conversion error: {}", e))?;
    Ok(next.to_rfc3339())
}

/// Calculate the duration until the next run based on schedule type and value.
/// All time calculations use the user's local timezone.
fn calculate_next_duration(
    schedule_type: &str,
    schedule_value: &str,
) -> Result<std::time::Duration, String> {
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
        "daily" => {
            // schedule_value = "HH:MM" in 24h format (e.g. "17:00")
            let (hour, minute) = parse_time_value(schedule_value)?;
            duration_until_next_daily(hour, minute)
        }
        "weekdays" => {
            // schedule_value = "HH:MM" in 24h format, runs Mon–Fri only
            let (hour, minute) = parse_time_value(schedule_value)?;
            duration_until_next_weekday(hour, minute)
        }
        "specific_days" => {
            // schedule_value = "mon,thu|15:00" (pipe-separated: days|time)
            let parts: Vec<&str> = schedule_value.split('|').collect();
            if parts.len() != 2 {
                return Err(format!(
                    "Invalid specific_days value: {}. Expected 'day1,day2|HH:MM'",
                    schedule_value
                ));
            }
            let days_str = parts[0].trim();
            let time_str = parts[1].trim();
            let (hour, minute) = parse_time_value(time_str)?;

            let target_weekdays: Vec<chrono::Weekday> = days_str
                .split(',')
                .map(|d| parse_weekday(d.trim()))
                .collect::<Result<Vec<_>, _>>()?;

            if target_weekdays.is_empty() {
                return Err("No days specified for specific_days schedule".to_string());
            }

            duration_until_next_specific_day(&target_weekdays, hour, minute)
        }
        _ => Err(format!("Unknown schedule type: {}", schedule_type)),
    }
}

/// Parse "HH:MM" format into (hour, minute).
fn parse_time_value(time_str: &str) -> Result<(u32, u32), String> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid time format: {}. Expected HH:MM", time_str));
    }
    let hour: u32 = parts[0]
        .parse()
        .map_err(|_| format!("Invalid hour in: {}", time_str))?;
    let minute: u32 = parts[1]
        .parse()
        .map_err(|_| format!("Invalid minute in: {}", time_str))?;
    if hour > 23 || minute > 59 {
        return Err(format!("Time out of range: {}", time_str));
    }
    Ok((hour, minute))
}

/// Parse a weekday string into a chrono::Weekday.
fn parse_weekday(s: &str) -> Result<chrono::Weekday, String> {
    match s.to_lowercase().as_str() {
        "monday" | "mon" => Ok(chrono::Weekday::Mon),
        "tuesday" | "tue" => Ok(chrono::Weekday::Tue),
        "wednesday" | "wed" => Ok(chrono::Weekday::Wed),
        "thursday" | "thu" => Ok(chrono::Weekday::Thu),
        "friday" | "fri" => Ok(chrono::Weekday::Fri),
        "saturday" | "sat" => Ok(chrono::Weekday::Sat),
        "sunday" | "sun" => Ok(chrono::Weekday::Sun),
        _ => Err(format!("Invalid weekday: {}", s)),
    }
}

/// Duration until the next occurrence of a daily HH:MM in local time.
fn duration_until_next_daily(hour: u32, minute: u32) -> Result<std::time::Duration, String> {
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
        today_target + chrono::Duration::days(1)
    };

    (target - now)
        .to_std()
        .map_err(|e| format!("Duration error: {}", e))
}

/// Duration until the next weekday (Mon–Fri) occurrence of HH:MM.
fn duration_until_next_weekday(hour: u32, minute: u32) -> Result<std::time::Duration, String> {
    let now = chrono::Local::now();
    for days_ahead in 0..=7 {
        let candidate_date = now.date_naive() + chrono::Duration::days(days_ahead);
        let weekday = candidate_date.weekday();
        if weekday == chrono::Weekday::Sat || weekday == chrono::Weekday::Sun {
            continue;
        }
        let candidate_dt = candidate_date
            .and_hms_opt(hour, minute, 0)
            .ok_or("Invalid time")?;
        let candidate = chrono::Local
            .from_local_datetime(&candidate_dt)
            .single()
            .ok_or("Ambiguous local time")?;

        if candidate > now {
            return (candidate - now)
                .to_std()
                .map_err(|e| format!("Duration error: {}", e));
        }
    }
    Err("Could not find next weekday occurrence".to_string())
}

/// Duration until the next occurrence of one of the specified weekdays at HH:MM.
fn duration_until_next_specific_day(
    target_days: &[chrono::Weekday],
    hour: u32,
    minute: u32,
) -> Result<std::time::Duration, String> {
    let now = chrono::Local::now();
    for days_ahead in 0..=7 {
        let candidate_date = now.date_naive() + chrono::Duration::days(days_ahead);
        let weekday = candidate_date.weekday();
        if !target_days.contains(&weekday) {
            continue;
        }
        let candidate_dt = candidate_date
            .and_hms_opt(hour, minute, 0)
            .ok_or("Invalid time")?;
        let candidate = chrono::Local
            .from_local_datetime(&candidate_dt)
            .single()
            .ok_or("Ambiguous local time")?;

        if candidate > now {
            return (candidate - now)
                .to_std()
                .map_err(|e| format!("Duration error: {}", e));
        }
    }
    Err("Could not find next occurrence for specified days".to_string())
}
