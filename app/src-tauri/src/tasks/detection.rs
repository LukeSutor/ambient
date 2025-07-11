use crate::tasks::models::{
    TaskStep, TaskDetectionResult, ScreenContext
};
use crate::models::llm::prompts::get_prompt;
use serde_json;

pub struct TaskDetectionService;

impl TaskDetectionService {
    /// Build a prompt for the LLM to detect task completion based on screen content
    pub fn build_detection_prompt(
        current_steps: &[TaskStep],
        screen_context: &ScreenContext,
    ) -> String {
        let steps_description = Self::format_steps_for_prompt(current_steps);
        
        let template = get_prompt("TASK_DETECTION")
            .expect("TASK_DETECTION prompt not found in prompts registry");
        
        template
            .replace("{steps}", &steps_description)
            .replace("{app}", &screen_context.application)
            .replace("{window_title}", screen_context.window_title.as_deref().unwrap_or("Unknown"))
            .replace("{screen_text}", &Self::truncate_screen_text(&screen_context.text, 2000))
    }

    /// Format task steps for inclusion in the LLM prompt
    fn format_steps_for_prompt(steps: &[TaskStep]) -> String {
        if steps.is_empty() {
            return "No active steps to monitor.".to_string();
        }

        steps
            .iter()
            .map(|step| {
                format!(
                    "Step ID: {}\nStep {}: {}\nDescription: {}\nCurrent Status: {}\n",
                    step.id,
                    step.step_number,
                    step.title,
                    step.description.as_deref().unwrap_or("No description"),
                    step.status
                )
            })
            .collect::<Vec<_>>()
            .join("\n---\n")
    }

    /// Truncate screen text to avoid overwhelming the LLM context
    fn truncate_screen_text(text: &str, max_length: usize) -> String {
        if text.len() <= max_length {
            text.to_string()
        } else {
            let truncated = &text[..max_length];
            format!("{}... [TRUNCATED - showing first {} chars of {} total]", 
                    truncated, max_length, text.len())
        }
    }

    /// Parse LLM response and validate the JSON structure
    pub fn parse_detection_response(response: &str) -> Result<TaskDetectionResult, String> {
        // Try to extract JSON from the response if it's wrapped in other text
        let json_str = Self::extract_json_from_response(response)?;
        
        match serde_json::from_str::<TaskDetectionResult>(&json_str) {
            Ok(result) => {
                // Validate the result structure
                Self::validate_detection_result(&result)?;
                Ok(result)
            }
            Err(e) => Err(format!("Failed to parse LLM response as JSON: {}", e))
        }
    }

    /// Extract JSON from LLM response that might contain extra text
    fn extract_json_from_response(response: &str) -> Result<String, String> {
        // Look for JSON object boundaries
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                if end > start {
                    return Ok(response[start..=end].to_string());
                }
            }
        }
        
        // If no JSON boundaries found, return original response and let JSON parser handle it
        Ok(response.to_string())
    }

    /// Validate the detection result structure and data
    fn validate_detection_result(result: &TaskDetectionResult) -> Result<(), String> {
        // Check completed steps
        for step in &result.completed_steps {
            if step.confidence < 0.0 || step.confidence > 1.0 {
                return Err(format!("Invalid confidence score for step {}: {}", step.step_id, step.confidence));
            }
            if step.evidence.trim().is_empty() {
                return Err(format!("Empty evidence for completed step {}", step.step_id));
            }
            if step.reasoning.trim().is_empty() {
                return Err(format!("Empty reasoning for completed step {}", step.step_id));
            }
        }

        // Check in-progress steps
        for step in &result.in_progress_steps {
            if step.confidence < 0.0 || step.confidence > 1.0 {
                return Err(format!("Invalid confidence score for in-progress step {}: {}", step.step_id, step.confidence));
            }
            if step.evidence.trim().is_empty() {
                return Err(format!("Empty evidence for in-progress step {}", step.step_id));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_truncate_screen_text() {
        let long_text = "a".repeat(3000);
        let truncated = TaskDetectionService::truncate_screen_text(&long_text, 2000);
        assert!(truncated.len() > 2000); // Should include truncation message
        assert!(truncated.contains("TRUNCATED"));
    }

    #[test]
    fn test_extract_json_from_response() {
        let response = "Here's the analysis: {\"completed_steps\": [], \"in_progress_steps\": []} That's my conclusion.";
        let json = TaskDetectionService::extract_json_from_response(response).unwrap();
        assert_eq!(json, r#"{"completed_steps": [], "in_progress_steps": []}"#);
    }

    #[test]
    fn test_format_steps_for_prompt() {
        let steps = vec![
            TaskStep {
                id: 1,
                task_id: 1,
                step_number: 1,
                title: "Test Step".to_string(),
                description: Some("Test description".to_string()),
                status: "pending".to_string(),
                completed_at: None,
            }
        ];

        let formatted = TaskDetectionService::format_steps_for_prompt(&steps);
        assert!(formatted.contains("Test Step"));
        assert!(formatted.contains("Test description"));
    }
}
