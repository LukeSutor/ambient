use crate::tasks::models::{
    TaskStep, TaskDetectionResult, CompletedStepDetection, InProgressStepDetection, ScreenContext
};
use serde_json;

pub struct TaskDetectionService;

impl TaskDetectionService {
    /// Build a prompt for the LLM to detect task completion based on screen content
    pub fn build_detection_prompt(
        current_steps: &[TaskStep],
        screen_context: &ScreenContext,
    ) -> String {
        let steps_description = Self::format_steps_for_prompt(current_steps);
        
        format!(
            r#"You are a task completion detection system. Analyze the current screen content to determine if any task steps have been completed or are in progress.

ACTIVE TASK STEPS TO MONITOR:
{steps}

CURRENT SCREEN INFORMATION:
Application: {app}
Window Title: {window_title}
Screen Text Content:
{screen_text}

INSTRUCTIONS:
1. Carefully analyze the screen content against each active task step
2. Look for evidence that matches the completion criteria for each step
3. Consider the application context - steps should only be marked complete if they occur in the expected application
4. Provide confidence scores between 0.0 and 1.0 (only mark as completed if confidence >= 0.8)
5. Include specific evidence from the screen that supports your decision

Respond with valid JSON in this exact format:
{{
  "completed_steps": [
    {{
      "step_id": <number>,
      "confidence": <0.0-1.0>,
      "evidence": "<specific text or elements from screen that indicate completion>",
      "reasoning": "<explain why this step is considered complete>"
    }}
  ],
  "in_progress_steps": [
    {{
      "step_id": <number>,
      "confidence": <0.0-1.0>,
      "evidence": "<indicators of partial progress or setup>"
    }}
  ],
  "suggestions": "<optional guidance for the user or null>"
}}

Only include steps in the response if there is clear evidence. If no steps are completed or in progress, return empty arrays.
"#,
            steps = steps_description,
            app = screen_context.application,
            window_title = screen_context.window_title.as_deref().unwrap_or("Unknown"),
            screen_text = Self::truncate_screen_text(&screen_context.text, 2000)
        )
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
                    "Step ID: {}\nStep {}: {}\nDescription: {}\nCompletion Criteria: {}\nExpected Application: {}\nCurrent Status: {}\n",
                    step.id,
                    step.step_number,
                    step.title,
                    step.description.as_deref().unwrap_or("No description"),
                    step.completion_criteria,
                    step.application_context.as_deref().unwrap_or("Any application"),
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

    /// Create a simplified prompt for testing or when screen context is minimal
    pub fn build_simple_prompt(
        step_title: &str,
        completion_criteria: &str,
        screen_text: &str,
        application: &str,
    ) -> String {
        format!(
            r#"Determine if this task step has been completed based on the screen content.

TASK STEP: {}
COMPLETION CRITERIA: {}
CURRENT APPLICATION: {}
SCREEN CONTENT: {}

Has this step been completed? Respond with JSON:
{{
  "completed": true/false,
  "confidence": 0.0-1.0,
  "evidence": "specific evidence from screen",
  "reasoning": "explanation of decision"
}}"#,
            step_title,
            completion_criteria,
            application,
            Self::truncate_screen_text(screen_text, 1000)
        )
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
                completion_criteria: "Test criteria".to_string(),
                application_context: Some("Chrome".to_string()),
                status: "pending".to_string(),
                completed_at: None,
            }
        ];

        let formatted = TaskDetectionService::format_steps_for_prompt(&steps);
        assert!(formatted.contains("Test Step"));
        assert!(formatted.contains("Test criteria"));
        assert!(formatted.contains("Chrome"));
    }
}
