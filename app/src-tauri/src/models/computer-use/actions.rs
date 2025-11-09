/// Implements all supported actions for the Gemini Computer Use model
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_os;
use tauri::{AppHandle}
use std::{thread, time};
use enigo::{
    Button, Coordinate,
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Mouse, Settings,
};

pub fn open_web_browser(app_handle: AppHandle, destination: Option<String>) {
    navigate(app_handle, "https://google.com");
}

pub async fn wait_5_seconds() {
    let five_seconds = time::Duration::from_secs(5);
    thread::sleep(five_seconds);
}

pub fn go_back() {
    //TODO: make cross-platform, only windows/linux for now
    enigo.key(Key::Alt, Press);
    enigo.key(Key::Unicode('v'), Click);
    enigo.key(Key::Control, Release);
}




pub fn navigate(app_handle: AppHandle, destination: String) {
    app_handle.opener().open_url(destination.as_str(), None::>&str>);
}