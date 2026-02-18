//! Semantic/event-based trigger evaluation for automations.
//!
//! Monitors screen content via OCR polling and evaluates trigger conditions
//! against the current screen state. Uses a two-stage approach:
//! 1. Fast keyword matching (free, instant)
//! 2. LLM classification (only if keywords match, expensive)

use super::types::AutomationTask;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;
use tokio::sync::RwLock;

/// Global flag to control the screen monitor loop.
static MONITOR_RUNNING: Lazy<Arc<AtomicBool>> =
    Lazy::new(|| Arc::new(AtomicBool::new(false)));

/// Cached OCR result with timestamp.
struct OcrCache {
    text: String,
    captured_at: std::time::Instant,
}

/// Global OCR cache.
static OCR_CACHE: Lazy<RwLock<Option<OcrCache>>> = Lazy::new(|| RwLock::new(None));

/// Default polling interval in seconds.
const DEFAULT_POLL_INTERVAL: u64 = 30;

/// OCR cache TTL in seconds.
const OCR_CACHE_TTL: u64 = 60;

/// Minimum cooldown between trigger firings for the same task (seconds).
const TRIGGER_COOLDOWN: u64 = 60;

/// Last trigger fire times per task_id.
static TRIGGER_COOLDOWNS: Lazy<RwLock<std::collections::HashMap<String, std::time::Instant>>> =
    Lazy::new(|| RwLock::new(std::collections::HashMap::new()));

/// Start the screen monitor for semantic triggers.
///
/// This spawns a background task that polls the screen via OCR
/// at a configurable interval and evaluates semantic triggers.
pub async fn start_screen_monitor(app_handle: &AppHandle) {
    if MONITOR_RUNNING.load(Ordering::SeqCst) {
        log::info!("[triggers] Screen monitor already running");
        return;
    }

    // Check if there are any enabled semantic tasks
    let semantic_tasks = match super::db::get_enabled_semantic_tasks(app_handle) {
        Ok(tasks) => tasks,
        Err(e) => {
            log::warn!("[triggers] Failed to load semantic tasks: {}", e);
            return;
        }
    };

    if semantic_tasks.is_empty() {
        log::info!("[triggers] No enabled semantic tasks, skipping screen monitor");
        return;
    }

    MONITOR_RUNNING.store(true, Ordering::SeqCst);
    let app = app_handle.clone();

    tokio::spawn(async move {
        log::info!("[triggers] Screen monitor started");

        loop {
            if !MONITOR_RUNNING.load(Ordering::SeqCst) {
                log::info!("[triggers] Screen monitor stopped");
                break;
            }

            // Sleep for the polling interval (read from settings, clamped to [5, 300] seconds)
            let poll_secs = {
                let settings = crate::settings::service::load_user_settings(app.clone()).await;
                settings
                    .map(|s| s.screen_poll_interval_secs.unwrap_or(DEFAULT_POLL_INTERVAL))
                    .unwrap_or(DEFAULT_POLL_INTERVAL)
                    .clamp(5, 300)
            };
            tokio::time::sleep(tokio::time::Duration::from_secs(poll_secs)).await;

            if !MONITOR_RUNNING.load(Ordering::SeqCst) {
                break;
            }

            // Re-check for enabled semantic tasks
            let tasks = match super::db::get_enabled_semantic_tasks(&app) {
                Ok(t) => t,
                Err(e) => {
                    log::warn!("[triggers] Failed to load semantic tasks: {}", e);
                    continue;
                }
            };

            if tasks.is_empty() {
                continue;
            }

            // Get screen text (from cache or fresh OCR)
            let screen_text = match get_screen_text(&app).await {
                Ok(text) => text,
                Err(e) => {
                    log::warn!("[triggers] Failed to get screen text: {}", e);
                    continue;
                }
            };

            // Evaluate each trigger
            for task in &tasks {
                if let Some(ref trigger_config) = task.trigger_config {
                    if should_trigger(task, &screen_text, trigger_config).await {
                        // Check cooldown
                        let cooldowns = TRIGGER_COOLDOWNS.read().await;
                        if let Some(last_fire) = cooldowns.get(&task.id) {
                            if last_fire.elapsed().as_secs() < TRIGGER_COOLDOWN {
                                continue;
                            }
                        }
                        drop(cooldowns);

                        // Update cooldown
                        {
                            let mut cooldowns = TRIGGER_COOLDOWNS.write().await;
                            cooldowns.insert(task.id.clone(), std::time::Instant::now());
                        }

                        // Execute the automation
                        log::info!(
                            "[triggers] Trigger fired for task '{}' ({})",
                            task.name,
                            task.id
                        );

                        let app_clone = app.clone();
                        let task_clone = task.clone();
                        tokio::spawn(async move {
                            match super::executor::execute_automation(&app_clone, &task_clone).await {
                                Ok(run) => {
                                    log::info!(
                                        "[triggers] Task '{}' completed: {}",
                                        task_clone.id,
                                        run.status
                                    );
                                }
                                Err(e) => {
                                    log::error!(
                                        "[triggers] Task '{}' execution failed: {}",
                                        task_clone.id,
                                        e
                                    );
                                }
                            }
                        });
                    }
                }
            }
        }
    });
}

/// Stop the screen monitor.
pub async fn stop_screen_monitor() {
    MONITOR_RUNNING.store(false, Ordering::SeqCst);
    log::info!("[triggers] Screen monitor stop requested");
}

/// Restart the screen monitor.
///
/// Stops the current monitor (if running) and starts a fresh one.
/// This is useful when automation tasks are added, removed, or changed.
pub async fn restart_screen_monitor(app_handle: &AppHandle) {
    stop_screen_monitor().await;
    // Give the running loop a moment to observe the stop signal.
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    start_screen_monitor(app_handle).await;
}

/// Get current screen text, using cache if available and fresh.
async fn get_screen_text(app_handle: &AppHandle) -> Result<String, String> {
    // Check cache first
    {
        let cache = OCR_CACHE.read().await;
        if let Some(ref cached) = *cache {
            if cached.captured_at.elapsed().as_secs() < OCR_CACHE_TTL {
                return Ok(cached.text.clone());
            }
        }
    }

    // Capture screen and run OCR
    let text = match capture_screen_text(app_handle).await {
        Ok(t) => t,
        Err(e) => {
            log::warn!("[triggers] Screen capture failed: {}", e);
            return Err(e);
        }
    };

    // Update cache
    {
        let mut cache = OCR_CACHE.write().await;
        *cache = Some(OcrCache {
            text: text.clone(),
            captured_at: std::time::Instant::now(),
        });
    }

    Ok(text)
}

/// Capture the primary screen and extract text via OCR.
async fn capture_screen_text(app_handle: &AppHandle) -> Result<String, String> {
    use screenshots::Screen;

    // Capture the primary screen
    let screens = Screen::all().map_err(|e| format!("Failed to enumerate screens: {}", e))?;
    let primary = screens
        .into_iter()
        .find(|s| s.display_info.is_primary)
        .or_else(|| {
            let all = Screen::all().ok()?;
            all.into_iter().next()
        })
        .ok_or("No screens available")?;

    let image_data = primary
        .capture()
        .map_err(|e| format!("Failed to capture screen: {}", e))?;

    // The screenshots crate may use a different version of the `image` crate.
    // Get the raw RGBA bytes and dimensions, then re-create an image with our version.
    let width = image_data.width();
    let height = image_data.height();
    let raw_bytes = image_data.into_raw();
    let our_image = image::RgbaImage::from_raw(width, height, raw_bytes)
        .ok_or("Failed to reconstruct image from raw bytes")?;

    // Encode as PNG bytes
    let mut png_bytes: Vec<u8> = Vec::new();
    let cursor = std::io::Cursor::new(&mut png_bytes);
    let encoder = image::codecs::png::PngEncoder::new(cursor);
    image::ImageEncoder::write_image(
        encoder,
        our_image.as_raw(),
        width,
        height,
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    // Run OCR
    let result = crate::models::ocr::ocr::process_image(app_handle.clone(), png_bytes)
        .await
        .map_err(|e| format!("OCR failed: {}", e))?;

    Ok(result.text)
}

/// Evaluate whether a trigger should fire based on screen text.
///
/// Uses a two-stage approach:
/// 1. Fast keyword matching
/// 2. LLM classification (future enhancement)
async fn should_trigger(
    task: &AutomationTask,
    screen_text: &str,
    trigger_config: &str,
) -> bool {
    let config: serde_json::Value = match serde_json::from_str(trigger_config) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let trigger_type = task.trigger_type.as_deref().unwrap_or("screen_content");

    match trigger_type {
        "screen_content" => {
            // Check keywords
            if let Some(keywords) = config.get("keywords").and_then(|k| k.as_array()) {
                let screen_lower = screen_text.to_lowercase();
                for keyword in keywords {
                    if let Some(kw) = keyword.as_str() {
                        if screen_lower.contains(&kw.to_lowercase()) {
                            return true;
                        }
                    }
                }
            }
            false
        }
        "url_visit" => {
            // Check for URL patterns in screen text
            if let Some(patterns) = config.get("url_patterns").and_then(|p| p.as_array()) {
                let screen_lower = screen_text.to_lowercase();
                for pattern in patterns {
                    if let Some(pat) = pattern.as_str() {
                        if screen_lower.contains(&pat.to_lowercase()) {
                            return true;
                        }
                    }
                }
            }
            false
        }
        _ => false,
    }
}
