use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle
};
use image::GenericImageView;
use crate::models;
use crate::windows;

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    // Use embedded icon bytes instead of file path to avoid path resolution issues
    let icon_bytes = include_bytes!("../../public/favicon.ico");
    
    // For ICO files, we need to decode them first
    let icon = match image::load_from_memory(icon_bytes) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            let (width, height) = img.dimensions();
            Image::new_owned(rgba.into_raw(), width, height)
        }
        Err(_) => {
            log::warn!("[tray] Failed to load favicon.ico, using default icon");
            // Fall back to a simple 32x32 transparent icon
            let rgba = vec![0; 32 * 32 * 4]; // Transparent pixels
            Image::new_owned(rgba, 32, 32)
        }
    };

    // Create menu items
    let open_hud = MenuItem::with_id(app, "open_hud", "Open HUD", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    
    // Create menu
    let menu = Menu::with_items(app, &[&open_hud, &quit])?;

    // Create tray icon
    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("TaskAware Assistant")
        .on_menu_event({
            let app_handle = app.clone();
            move |_app, event| {
                match event.id.as_ref() {
                    "open_hud" => {
                        log::info!("[tray] Opening HUD window");
                        if let Err(e) = tauri::async_runtime::block_on(
                            windows::open_floating_window(app_handle.clone(), Some("floating-hud".to_string()))
                        ) {
                            log::error!("[tray] Failed to open HUD window: {}", e);
                        }
                    },
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
                    },
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
                    TrayIconEvent::Click { button, button_state, .. } => {
                        // Do nothing for now...
                    },
                    TrayIconEvent::DoubleClick { button, .. } => {
                        log::info!("[tray] Tray double-clicked: {:?}", button);
                        // Open HUD on double-click
                        if let Err(e) = tauri::async_runtime::block_on(
                            windows::open_floating_window(app_handle.clone(), Some("floating-hud".to_string()))
                        ) {
                            log::error!("[tray] Failed to open HUD window on double-click: {}", e);
                        }
                    },
                    _ => {}
                }
            }
        })
        .build(app)?;

    log::info!("[tray] System tray created successfully");
    Ok(())
}
