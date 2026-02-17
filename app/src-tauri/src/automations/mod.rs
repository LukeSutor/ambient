//! Automations: time-based and event-driven task execution.
//!
//! # Architecture
//!
//! - `types`: Core enums and data structures
//! - `db`: Database CRUD operations
//! - `commands`: Tauri commands exposed to the frontend
//! - `events`: Event constants and payload types
//! - `scheduler`: tokio-based time scheduling engine
//! - `triggers`: OCR-based screen monitoring for semantic triggers
//! - `executor`: Background agent execution engine
//! - `notifications`: HUD notification delivery
//! - `system_tasks`: Built-in system automation templates

pub mod commands;
pub mod db;
pub mod events;
pub mod executor;
pub mod notifications;
pub mod scheduler;
pub mod system_tasks;
pub mod triggers;
pub mod types;
