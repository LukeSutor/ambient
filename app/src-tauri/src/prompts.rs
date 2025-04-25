use once_cell::sync::Lazy;
use std::collections::HashMap;

// Use Lazy to initialize the HashMap only once
static PROMPTS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(
        "SUMMARIZE_ACTION",
        "You are a computer screenshot analysis expert. You will be given an screenshot of a person using a computer, and you must accurately and precisely describe what they are currently doing based on the screenshot. Your response should be short and sweet, optimized for creating an embedding for document similarity with other tasks the user does. What is the user doing in this image?",
    );
    // Add more prompts here as needed
    // map.insert("ANOTHER_KEY", "Another prompt text.");
    map
});

/// Fetches a prompt by its key.
/// Internal function, not directly exposed as a Tauri command.
pub fn get_prompt(key: &str) -> Option<&'static str> {
    PROMPTS.get(key).copied()
}

/// Tauri command to fetch a prompt by its key.
#[tauri::command]
pub fn get_prompt_command(key: String) -> Result<String, String> {
    match get_prompt(&key) {
        Some(prompt) => Ok(prompt.to_string()),
        None => Err(format!("Prompt with key '{}' not found.", key)),
    }
}
