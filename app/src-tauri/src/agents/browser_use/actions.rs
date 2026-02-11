//! Browser action execution via WebView JavaScript injection.
//!
//! Executes browser actions (click, type, navigate, etc.) by injecting
//! JavaScript into the browser-use WebView via `eval()`. Actions are
//! fire-and-forget; verification comes from the subsequent DOM snapshot.

use std::time::Duration;
use tauri::{AppHandle, Manager};
use serde_json::Value;

use super::webview::get_browser_window_label;

/// Execute a browser action on the WebView.
///
/// Returns a brief description of the action taken (for logging/display).
pub async fn execute_action(
    app_handle: &AppHandle,
    tool_name: &str,
    arguments: &Value,
) -> Result<String, String> {
    match tool_name {
        "navigate" => execute_navigate(app_handle, arguments).await,
        "click" => execute_click(app_handle, arguments).await,
        "type_text" => execute_type_text(app_handle, arguments).await,
        "select_option" => execute_select_option(app_handle, arguments).await,
        "scroll" => execute_scroll(app_handle, arguments).await,
        "go_back" => execute_go_back(app_handle).await,
        "wait" => execute_wait(arguments).await,
        _ => Err(format!("Unknown browser action: {}", tool_name)),
    }
}

/// Navigate to a URL.
async fn execute_navigate(app_handle: &AppHandle, args: &Value) -> Result<String, String> {
    let url = args
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'url' argument")?;

    let webview = app_handle
        .get_webview_window(get_browser_window_label())
        .ok_or("Browser WebView not found")?;

    let js = format!(
        r#"(function(){{ window.location.href = '{}'; }})()"#,
        url.replace('\'', "\\'")
    );

    webview
        .eval(&js)
        .map_err(|e| format!("Failed to navigate: {}", e))?;

    log::info!("[browser_use] Navigating to: {}", url);
    Ok(format!("Navigating to {}", url))
}

/// Click an element by its snapshot ID.
async fn execute_click(app_handle: &AppHandle, args: &Value) -> Result<String, String> {
    let element_id = args
        .get("element_id")
        .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
        .ok_or("Missing or invalid 'element_id' argument")?;

    let webview = app_handle
        .get_webview_window(get_browser_window_label())
        .ok_or("Browser WebView not found")?;

    let js = format!(
        r#"(function(){{
            var el = window.__elements && window.__elements[{idx}];
            if (!el) {{
                console.error('[browser_use] Element {id} not found');
                return;
            }}
            // Scroll element into view if needed
            el.scrollIntoView({{ block: 'center', behavior: 'instant' }});
            // Try clicking
            el.focus();
            el.click();
            console.log('[browser_use] Clicked element {id}');
        }})()"#,
        idx = element_id - 1,
        id = element_id
    );

    webview
        .eval(&js)
        .map_err(|e| format!("Failed to click element: {}", e))?;

    log::info!("[browser_use] Clicked element [{}]", element_id);
    Ok(format!("Clicked element [{}]", element_id))
}

/// Type text into an input element.
async fn execute_type_text(app_handle: &AppHandle, args: &Value) -> Result<String, String> {
    let element_id = args
        .get("element_id")
        .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
        .ok_or("Missing or invalid 'element_id' argument")?;

    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'text' argument")?;

    let press_enter = args
        .get("press_enter")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let webview = app_handle
        .get_webview_window(get_browser_window_label())
        .ok_or("Browser WebView not found")?;

    let escaped_text = text
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r");

    let enter_js = if press_enter {
        r#"
            setTimeout(function() {
                el.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', keyCode: 13, bubbles: true }));
                el.dispatchEvent(new KeyboardEvent('keypress', { key: 'Enter', code: 'Enter', keyCode: 13, bubbles: true }));
                el.dispatchEvent(new KeyboardEvent('keyup', { key: 'Enter', code: 'Enter', keyCode: 13, bubbles: true }));
                if (el.form) { el.form.submit(); }
            }, 100);
        "#
    } else {
        ""
    };

    let js = format!(
        r#"(function(){{
            var el = window.__elements && window.__elements[{idx}];
            if (!el) {{
                console.error('[browser_use] Element {id} not found');
                return;
            }}
            el.focus();
            // Clear existing value
            el.value = '';
            el.dispatchEvent(new Event('input', {{ bubbles: true }}));
            // Set new value
            el.value = '{text}';
            el.dispatchEvent(new Event('input', {{ bubbles: true }}));
            el.dispatchEvent(new Event('change', {{ bubbles: true }}));
            console.log('[browser_use] Typed into element {id}');
            {enter_js}
        }})()"#,
        idx = element_id - 1,
        id = element_id,
        text = escaped_text,
        enter_js = enter_js
    );

    webview
        .eval(&js)
        .map_err(|e| format!("Failed to type text: {}", e))?;

    let msg = if press_enter {
        format!("Typed '{}' into element [{}] and pressed Enter", text, element_id)
    } else {
        format!("Typed '{}' into element [{}]", text, element_id)
    };
    log::info!("[browser_use] {}", msg);
    Ok(msg)
}

/// Select an option from a dropdown.
async fn execute_select_option(app_handle: &AppHandle, args: &Value) -> Result<String, String> {
    let element_id = args
        .get("element_id")
        .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
        .ok_or("Missing or invalid 'element_id' argument")?;

    let value = args
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'value' argument")?;

    let webview = app_handle
        .get_webview_window(get_browser_window_label())
        .ok_or("Browser WebView not found")?;

    let escaped_value = value.replace('\\', "\\\\").replace('\'', "\\'");

    let js = format!(
        r#"(function(){{
            var el = window.__elements && window.__elements[{idx}];
            if (!el || el.tagName.toLowerCase() !== 'select') {{
                console.error('[browser_use] Select element {id} not found');
                return;
            }}
            // Try to match by value first, then by text
            var found = false;
            for (var i = 0; i < el.options.length; i++) {{
                if (el.options[i].value === '{val}' || el.options[i].textContent.trim() === '{val}') {{
                    el.selectedIndex = i;
                    found = true;
                    break;
                }}
            }}
            if (found) {{
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                console.log('[browser_use] Selected option in element {id}');
            }} else {{
                console.error('[browser_use] Option not found in element {id}');
            }}
        }})()"#,
        idx = element_id - 1,
        id = element_id,
        val = escaped_value
    );

    webview
        .eval(&js)
        .map_err(|e| format!("Failed to select option: {}", e))?;

    log::info!("[browser_use] Selected '{}' in element [{}]", value, element_id);
    Ok(format!("Selected '{}' in element [{}]", value, element_id))
}

/// Scroll the page up or down.
async fn execute_scroll(app_handle: &AppHandle, args: &Value) -> Result<String, String> {
    let direction = args
        .get("direction")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'direction' argument")?;

    let pixels = match direction {
        "up" => -500,
        "down" => 500,
        _ => return Err(format!("Invalid direction '{}', use 'up' or 'down'", direction)),
    };

    let webview = app_handle
        .get_webview_window(get_browser_window_label())
        .ok_or("Browser WebView not found")?;

    let js = format!(
        r#"(function(){{ window.scrollBy({{ top: {}, behavior: 'smooth' }}); }})()"#,
        pixels
    );

    webview
        .eval(&js)
        .map_err(|e| format!("Failed to scroll: {}", e))?;

    log::info!("[browser_use] Scrolled {}", direction);
    Ok(format!("Scrolled {}", direction))
}

/// Navigate back to the previous page.
async fn execute_go_back(app_handle: &AppHandle) -> Result<String, String> {
    let webview = app_handle
        .get_webview_window(get_browser_window_label())
        .ok_or("Browser WebView not found")?;

    webview
        .eval("(function(){ window.history.back(); })()")
        .map_err(|e| format!("Failed to go back: {}", e))?;

    log::info!("[browser_use] Navigated back");
    Ok("Navigated back".to_string())
}

/// Wait for a specified number of seconds.
async fn execute_wait(args: &Value) -> Result<String, String> {
    let seconds = args
        .get("seconds")
        .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
        .unwrap_or(2);

    let seconds = seconds.min(10); // Cap at 10 seconds
    tokio::time::sleep(Duration::from_secs(seconds)).await;

    log::info!("[browser_use] Waited {} seconds", seconds);
    Ok(format!("Waited {} seconds", seconds))
}
