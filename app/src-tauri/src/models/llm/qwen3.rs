use anyhow::Result;
use mistralrs::{
    TextMessageRole, TextMessages, GgufModelBuilder, TextModelBuilder, IsqType
};

/// Generates a response from the Qwen3-8B model given a user prompt.
/// This function is exposed as a Tauri command.
#[tauri::command]
pub async fn generate(prompt: String) -> Result<String, String> {
    println!("[Qwen3] Generating response for prompt: {}", prompt);
    let model = TextModelBuilder::new("C:\\Users\\Luke\\AppData\\Roaming\\com.tauri.dev\\models\\vlm\\qwen3")
        .with_isq(IsqType::Q4K)
        .with_logging()
        .build()
        .await
        .map_err(|e| e.to_string())?;

    println!("[Qwen3] Model loaded successfully.");

    let messages = TextMessages::new()
        .add_message(
            TextMessageRole::System,
            "You are a helpful assistant.",
        )
        .add_message(
            TextMessageRole::User,
            &prompt,
        );

    let response = model
        .send_chat_request(messages)
        .await
        .map_err(|e| e.to_string())?;

    let content = response.choices[0]
        .message
        .content
        .as_ref()
        .cloned()
        .unwrap_or_else(|| "".to_string());

    println!("[Qwen3] Response generated successfully: {}", content);

    Ok(content)
}