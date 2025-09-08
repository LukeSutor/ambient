use crate::db::{get_latest_activity_summary, insert_activity_summary, DbState};
use crate::events::{emitter::emit, types::*};
use crate::models::llm::{prompts::get_prompt, schemas::get_schema, client::generate};
use crate::tasks::{TaskService, TaskWithSteps};
use tauri::{AppHandle, Manager};