use crate::models;
use crate::windows;
use image::GenericImageView;
use tauri::{
  image::Image,
  menu::{Menu, MenuItem},
  tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
  AppHandle,
};

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
  // Use PNG for better quality - tray icons should be 32x32 on Windows
  let icon_bytes = include_bytes!("../icons/32x32.png");

  // PNG files can be loaded directly without quality loss
  let icon = match image::load_from_memory(icon_bytes) {
    Ok(img) => {
      let rgba = img.to_rgba8();
      let (width, height) = img.dimensions();
      log::info!("[tray] Loaded tray icon: {}x{}", width, height);
      Image::new_owned(rgba.into_raw(), width, height)
    }
    Err(e) => {
      log::error!("[tray] Failed to load 32x32.png: {}", e);
      // Fall back to a simple 32x32 transparent icon
      let rgba = vec![0; 32 * 32 * 4]; // Transparent pixels
      Image::new_owned(rgba, 32, 32)
    }
  };

  // Create menu items
  let open_hud = MenuItem::with_id(app, "open_hud", "Open", true, None::<&str>)?;
  let settings = MenuItem::with_id(app, "settings", "Dashboard", true, None::<&str>)?;
  let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

  // Create menu
  let menu = Menu::with_items(app, &[&open_hud, &settings, &quit])?;

  // Create tray icon
  let _tray = TrayIconBuilder::new()
    .icon(icon)
    .menu(&menu)
    .show_menu_on_left_click(false)
    .tooltip("Ambient Assistant")
    .on_menu_event({
      let app_handle = app.clone();
      move |_app, event| {
        match event.id.as_ref() {
          "open_hud" => {
            log::info!("[tray] Opening HUD window");
            if let Err(e) =
              tauri::async_runtime::block_on(windows::open_main_window(app_handle.clone()))
            {
              log::error!("[tray] Failed to open HUD window: {}", e);
            }
          }
          "settings" => {
            log::info!("[tray] Opening secondary window");
            if let Err(e) = tauri::async_runtime::block_on(windows::open_secondary_window(
              app_handle.clone(),
              None,
            )) {
              log::error!("[tray] Failed to open main window: {}", e);
            }
          }
          "quit" => {
            log::info!("[tray] Quit requested from tray");
            let app_handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
              // Stop the llama server gracefully
              if let Err(e) = models::llm::server::stop_llama_server().await {
                log::error!("[tray] Failed to stop llama server during quit: {}", e);
              } else {
                log::info!("[tray] Llama server stopped successfully");
              }

              // Exit the application
              app_handle.exit(0);
            });
          }
          _ => {
            log::warn!("[tray] Unknown menu event: {:?}", event.id);
          }
        }
      }
    })
    .on_tray_icon_event({
      let app_handle = app.clone();
      move |_tray, event| {
        match event {
          TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
          } => {
            log::info!("[tray] Tray left-clicked, opening HUD");
            if let Err(e) =
              tauri::async_runtime::block_on(windows::open_main_window(app_handle.clone()))
            {
              log::error!("[tray] Failed to open HUD window on click: {}", e);
            }
          }
          TrayIconEvent::DoubleClick { .. } => {
            log::info!("[tray] Tray double-clicked");
            // Open HUD on double-click
            if let Err(e) =
              tauri::async_runtime::block_on(windows::open_main_window(app_handle.clone()))
            {
              log::error!("[tray] Failed to open HUD window on double-click: {}", e);
            }
          }
          _ => {}
        }
      }
    })
    .build(app)?;

  Ok(())
}
