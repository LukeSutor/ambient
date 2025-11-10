/// Implements all supported actions for the Gemini Computer Use model
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_os;
use tauri::AppHandle;
use std::{thread, time};
use enigo::{
    Button, Coordinate, Axis,
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Mouse, Settings,
};

/// Maps a string representation of a key to the Enigo Key enum
fn map_key(key_str: &str) -> Option<Key> {
    match key_str.trim() {
        // Modifier keys
        "control" | "ctrl" => Some(Key::Control),
        "shift" => Some(Key::Shift),
        "alt" => Some(Key::Alt),
        "command" | "cmd" | "super" | "meta" | "windows" => Some(Key::Meta),
        
        // Navigation keys
        "enter" | "return" => Some(Key::Return),
        "tab" => Some(Key::Tab),
        "space" => Some(Key::Space),
        "backspace" => Some(Key::Backspace),
        "delete" | "del" => Some(Key::Delete),
        "escape" | "esc" => Some(Key::Escape),
        
        // Arrow keys
        "up" | "uparrow" => Some(Key::UpArrow),
        "down" | "downarrow" => Some(Key::DownArrow),
        "left" | "leftarrow" => Some(Key::LeftArrow),
        "right" | "rightarrow" => Some(Key::RightArrow),
        
        // Function keys
        "f1" => Some(Key::F1),
        "f2" => Some(Key::F2),
        "f3" => Some(Key::F3),
        "f4" => Some(Key::F4),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        "f9" => Some(Key::F9),
        "f10" => Some(Key::F10),
        "f11" => Some(Key::F11),
        "f12" => Some(Key::F12),
        
        // Other common keys
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "pageup" | "pgup" => Some(Key::PageUp),
        "pagedown" | "pgdn" => Some(Key::PageDown),
        "insert" => Some(Key::Insert),
        "capslock" => Some(Key::CapsLock),
        
        // Single character keys
        s if s.len() == 1 => s.chars().next().map(Key::Unicode),
        
        _ => None,
    }
}

/// Computer use actions

pub fn open_web_browser(app_handle: AppHandle) -> Result<(), String> {
    navigate(app_handle, "https://google.com").unwrap();
    Ok(())
}

pub async fn wait_5_seconds() -> Result<(), String> {
    let five_seconds = time::Duration::from_secs(5);
    thread::sleep(five_seconds);
    Ok(())
}

pub fn go_back() -> Result<(), String> {
    let platform: &str = tauri_plugin_os::platform();
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    let mut key = Key::Alt;
    if platform == "macos" {
        key = Key::Meta;
    }
    enigo.key(key, Press).unwrap();
    enigo.key(Key::LeftArrow, Click).unwrap();
    enigo.key(key, Release).unwrap();
    Ok(())
}

pub fn go_forward() -> Result<(), String> {
    let platform: &str = tauri_plugin_os::platform();
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    let mut key = Key::Alt;
    if platform == "macos" {
        key = Key::Meta;
    }
    enigo.key(key, Press).unwrap();
    enigo.key(Key::RightArrow, Click).unwrap();
    enigo.key(key, Release).unwrap();
    Ok(())
}

pub fn search(app_handle: AppHandle) -> Result<(), String> {
    open_web_browser(app_handle).unwrap();
    Ok(())
}

pub fn navigate(app_handle: AppHandle, url: &str) -> Result<(), String> {
    app_handle.opener().open_url(url, None::<&str>).unwrap();
    Ok(())
}

pub fn click_at(x: i32, y: i32) -> Result<(), String> {
    hover_at(x, y).unwrap();
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    enigo.button(Button::Left, Click).unwrap();
    Ok(())
}

pub fn hover_at(x: i32, y: i32) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    enigo.move_mouse(x, y, Coordinate::Abs).unwrap();
    Ok(())
}

pub fn type_text_at(x: i32, y: i32, text: &str, press_enter: Option<bool>, clear_before_typing: Option<bool>) -> Result<(), String> {
    let press_enter = press_enter.unwrap_or(true);
    let clear_before_typing = clear_before_typing.unwrap_or(true);

    // Select the text box
    click_at(x, y).unwrap();
    let mut enigo = Enigo::new(&Settings::default()).unwrap();

    // Optionally clear the text
    if clear_before_typing {
        let platform: &str = tauri_plugin_os::platform();
        let mut key = Key::Control;
        if platform == "macos" {
            key = Key::Meta;
        }
        enigo.key(key, Press).unwrap();
        enigo.key(Key::Unicode('a'), Click).unwrap();
        enigo.key(key, Release).unwrap();
        enigo.key(Key::Backspace, Click).unwrap();
    }

    // Type text
    enigo.text(text).unwrap();

    // Optionally press enter
    if press_enter {
        enigo.key(Key::Return, Click).unwrap();
    }
    Ok(())
}

pub fn key_combination(keys: &str) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
        
    // Parse the key combination string
    let keys_lower = keys.to_lowercase();
    let parts: Vec<&str> = keys_lower.split('+').collect();
    
    // Press all modifier keys first
    for i in 0..parts.len() - 1 {
        if let Some(key) = map_key(parts[i]) {
            enigo.key(key, Press).ok();
        }
    }
    
    // Click the final key
    if let Some(last_key) = parts.last().and_then(|k| map_key(k)) {
        enigo.key(last_key, Click).ok();
    }
    
    // Release all modifier keys in reverse order
    for i in (0..parts.len() - 1).rev() {
        if let Some(key) = map_key(parts[i]) {
            enigo.key(key, Release).ok();
        }
    }
    Ok(())
}

pub fn scroll_document(direction: &str) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    // Scroll 6 clicks by default
    let mut scroll_axis = Axis::Vertical;
    if direction == "left" || direction == "right" {
        scroll_axis = Axis::Horizontal;
    }
    let mut direction_multiplier = 1;
    if direction == "left" || direction == "down" {
        direction_multiplier = -1;
    }
    enigo.scroll(direction_multiplier * 6, scroll_axis).unwrap();
    Ok(())
}

pub fn scroll_at(x: i32, y: i32, direction: &str, magnitude: Option<i32>) -> Result<(), String> {
    let magnitude = magnitude.unwrap_or(800);
    // Normalize magnitude with 800 being 6 scrolls
    let normalized_magnitude = (magnitude as f64 / 800.0 * 6.0).round() as i32;

    hover_at(x, y).unwrap();
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    let mut scroll_axis = Axis::Vertical;
    if direction == "left" || direction == "right" {
        scroll_axis = Axis::Horizontal;
    }
    let mut direction_multiplier = 1;
    if direction == "left" || direction == "down" {
        direction_multiplier = -1;
    }
    enigo.scroll(direction_multiplier * normalized_magnitude, scroll_axis).unwrap();
    Ok(())
}

pub fn drag_and_drop(x: i32, y: i32, destination_x: i32, destination_y: i32) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    
    // Move to start position
    enigo.move_mouse(x, y, Coordinate::Abs).unwrap();
    
    // Press and hold left mouse button
    enigo.button(Button::Left, Press).unwrap();
    
    // Move to destination
    enigo.move_mouse(destination_x, destination_y, Coordinate::Abs).unwrap();
    
    // Release mouse button
    enigo.button(Button::Left, Release).unwrap();
    Ok(())
}