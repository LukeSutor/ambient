use once_cell::sync::Lazy;
use std::collections::HashMap;

// Use Lazy to initialize the HashMap only once
static SCHEMAS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
  let mut map = HashMap::new();
  map.insert(
    "detect_tasks",
    r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "analysis": {
      "type": "string",
      "description": "Reasoning behind which tasks have been completed"
    },
    "completed": {
      "type": "array",
      "items": {
        "type": "integer",
        "description": "IDs of tasks that have been completed"
      }
    }
  },
  "required": ["analysis", "completed"],
  "additionalProperties": false
}"#,
  );
  map.insert(
    "summarize_screen",
    r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "summary": {
      "type": "string",
      "description": "A concise 2-3 sentence summary of the user's current primary activity"
    }
  },
  "required": ["summary"],
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
