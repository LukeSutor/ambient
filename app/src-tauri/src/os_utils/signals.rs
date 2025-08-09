// Cross-platform signal handler setup, with Windows-specific console control handling.

#[cfg(windows)]
pub fn setup_signal_handlers() {
  use std::sync::atomic::AtomicBool;
  use std::sync::Arc;

  let _shutdown_flag = Arc::new(AtomicBool::new(false));

  extern "system" fn console_handler(ctrl_type: u32) -> i32 {
    match ctrl_type {
      0 => {
        // CTRL_C_EVENT
        log::info!("[signal] Received CTRL+C, shutting down llama server...");
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
          if let Err(e) = crate::models::llm::server::stop_llama_server().await {
            log::error!("[signal] Failed to stop llama server: {}", e);
          }
        });
        1 // TRUE - handled
      }
      2 => {
        // CTRL_BREAK_EVENT
        log::info!("[signal] Received CTRL+BREAK, shutting down llama server...");
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
          if let Err(e) = crate::models::llm::server::stop_llama_server().await {
            log::error!("[signal] Failed to stop llama server: {}", e);
          }
        });
        1
      }
      5 => {
        // CTRL_LOGOFF_EVENT
        log::info!("[signal] Received LOGOFF event, shutting down llama server...");
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
          if let Err(e) = crate::models::llm::server::stop_llama_server().await {
            log::error!("[signal] Failed to stop llama server: {}", e);
          }
        });
        1
      }
      6 => {
        // CTRL_SHUTDOWN_EVENT
        log::info!("[signal] Received SHUTDOWN event, shutting down llama server...");
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
          if let Err(e) = crate::models::llm::server::stop_llama_server().await {
            log::error!("[signal] Failed to stop llama server: {}", e);
          }
        });
        1
      }
      _ => 0, // not handled
    }
  }

  unsafe {
    type BOOL = i32;
    type DWORD = u32;

    #[link(name = "kernel32")]
    extern "system" {
      fn SetConsoleCtrlHandler(
        handler: Option<unsafe extern "system" fn(DWORD) -> BOOL>,
        add: BOOL,
      ) -> BOOL;
    }

    SetConsoleCtrlHandler(Some(console_handler), 1);
  }
}

#[cfg(not(windows))]
pub fn setup_signal_handlers() {
  log::info!("[signal] Signal handlers not implemented for this platform");
}
