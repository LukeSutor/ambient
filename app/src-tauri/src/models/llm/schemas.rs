use once_cell::sync::Lazy;
use std::collections::HashMap;

// Use Lazy to initialize the HashMap only once
static SCHEMAS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
  let mut map = HashMap::new();
  map.insert(
        "detect_tasks_response",
r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "updates": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "reasoning": {
            "type": "string",
            "description": "Explanation of why this task is relevant based on screen content"
          },
          "evidence": {
            "type": "string",
            "description": "Specific text or content from the screen that supports this conclusion"
          },
          "step_id": {
            "type": "integer",
            "description": "Unique identifier for the specific step within the task"
          },
          "status": {
            "type": "string",
            "enum": ["completed", "in_progress"],
            "description": "Current status of the task step"
          },
          "confidence": {
            "type": "number",
            "minimum": 0.0,
            "maximum": 1.0,
            "description": "Confidence level of the detection (0.0 to 1.0)"
          }
        },
        "required": ["reasoning", "evidence", "step_id", "status", "confidence"],
        "additionalProperties": false
      }
    }
  },
  "required": ["updates"],
  "additionalProperties": false
}"#,
    );
  map
});

/// Fetches a schema by its key.
pub fn get_schema(key: &str) -> Option<&'static str> {
  SCHEMAS.get(key).copied()
}

/// Tauri command to fetch a schema by its key.
#[tauri::command]
pub fn get_schema_command(key: String) -> Result<String, String> {
  match get_schema(&key) {
    Some(schema) => Ok(schema.to_string()),
    None => Err(format!("Schema with key '{}' not found.", key)),
  }
}
