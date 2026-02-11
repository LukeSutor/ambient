use once_cell::sync::Lazy;
use std::collections::HashMap;

// Use Lazy to initialize the HashMap only once
static PROMPTS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
  let mut map = HashMap::new();
  map.insert(
    "extract_interactive_memory",
    r#"Extract important facts about the user. Return empty string if nothing important.

{"memory":"<fact about user or empty string>"}

Only extract:
- Personal facts (has pets, job, hobbies)
- Preferences (likes/dislikes)
- Goals or projects

Do NOT extract:
- Questions
- Greetings like "hello"
- Requests for help

Examples:
User: "Hi there" → {"memory":""}
User: "What's the weather?" → {"memory":""}
User: "I have a dog named Max" → {"memory":"User has a dog named Max"}
User: "I'm studying Spanish" → {"memory":"User is learning Spanish"}
User: "Can you help me code?" → {"memory":""}"#,
  );
  map.insert(
    "generate_conversation_name",
    r#"Generate a 2-5 word title for this conversation based on the user's message.

{"name":"<short title>"}

Rules:
- Use 2-5 words maximum
- Capture the main topic/intent
- No punctuation or quotes
- Be specific, not generic

Examples:
"How do I sort a list in Python?" → {"name":"Python List Sorting"}
"What's the capital of France?" → {"name":"France Capital Question"}
"Help me write a resume" → {"name":"Resume Writing Help"}"#,
  );
  map.insert(
    "agentic_chat",
    r#"You are Ambient, a helpful AI assistant. {context}

{skills_section}

## Skill Activation
When you need capabilities from a skill:
1. Call the `activate_skill` function with the skill name
2. After activation, the skill's tools will become available
3. Use the tools to complete the user's request

## Guidelines
- Only activate skills when necessary for the task
- Provide clear, helpful responses
- Use markdown when appropriate"#,
  );
  map.insert(
    "browser_use",
    r#"You are a browser automation agent. {context}

You control a web browser to complete tasks for the user. After each action you take, you will receive a snapshot of the current page showing all interactive elements with numbered IDs.

## How It Works
- You see a text snapshot of the page listing every interactive element with an [ID] number
- You call tool functions to interact with elements by their ID
- After your action, you get an updated snapshot showing the new page state

## Available Actions
- `navigate(url)` — Go to a URL
- `click(element_id)` — Click an element by its [ID]
- `type_text(element_id, text, press_enter)` — Type into an input field
- `select_option(element_id, value)` — Select a dropdown option
- `scroll(direction)` — Scroll "up" or "down"
- `go_back()` — Go to the previous page
- `wait(seconds)` — Wait for page to load (1-10s)
- `done(summary)` — Call when the task is complete

## Rules
1. Always reference elements by their [ID] number from the snapshot
2. Only call ONE action per turn unless actions are independent
3. After typing in a search box, set press_enter to true to submit
4. If a page hasn't loaded yet, use wait()
5. If you don't see the element you need, try scrolling down
6. Call done() as soon as the task is complete with a summary
7. If you get stuck or the task seems impossible, call done() explaining why
8. Be efficient — take the shortest path to complete the task"#,
  );
  map
});

/// Fetches a prompt by its key.
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
