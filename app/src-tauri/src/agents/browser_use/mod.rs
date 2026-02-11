//! Browser-use agent module.
//!
//! Provides a browser automation agent that uses structure-first DOM snapshots
//! instead of vision/screenshots. The agent operates on a persistent Tauri
//! WebView and executes actions via JavaScript injection.

pub mod actions;
pub mod commands;
pub mod runtime;
pub mod snapshot;
pub mod state;
pub mod types;
pub mod webview;

pub use state::BrowserUseState;
