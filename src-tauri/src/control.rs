// Contains functions for interacting with the computer

use enigo::{Button, Coordinate, Direction::Click, Enigo, Keyboard, Mouse, Settings};
use screenshots::Screen;

#[tauri::command]
pub fn move_mouse(x: f64, y: f64) {
    // Moves the mouse to an absolute position on the screen

    // Translate the relative positions to absolute
    let screens = Screen::all().unwrap();
    let screen = &screens[0]; // Assuming single screen for simplicity
    let abs_x = (screen.display_info.width as f64 * x) as i32;
    let abs_y = (screen.display_info.height as f64 * y) as i32;

    // Move the mouse
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    if let Err(e) = enigo.move_mouse(abs_x, abs_y, Coordinate::Abs) {
        eprintln!("Failed to move mouse: {:?}", e);
    }
    println!("Clicked at location: {}, {}", abs_x, abs_y);
}

#[tauri::command]
pub fn click_mouse(button: String) {
    // Clicks the specified button on the mouse
    let mut enigo_button = Button::Left;
    if button == "RIGHT" {
        enigo_button = Button::Right;
    } else if button == "MIDDLE" {
        enigo_button = Button::Middle;
    }

    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    if let Err(e) = enigo.button(enigo_button, Click) {
        eprintln!("Failed to click mouse button: {:?}", e);
    }
}

#[tauri::command]
pub fn type_string(string: String) {
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    if let Err(e) = enigo.text(&string) {
        eprintln!("Failed to type text '{}': {:?}", string, e);
    }
}
