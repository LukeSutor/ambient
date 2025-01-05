use enigo::{Button, Coordinate, Direction::Click, Enigo, Key, Keyboard, Mouse, Settings};

#[tauri::command]
pub fn move_mouse(x: i32, y: i32) {
    // Moves the mouse to an absolute position on the screen
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    if let Err(e) = enigo.move_mouse(x, y, Coordinate::Abs) {
        eprintln!("Failed to move mouse: {:?}", e);
    }
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
