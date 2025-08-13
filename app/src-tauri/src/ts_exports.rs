use crate::settings::{HudDimensions, HudSizeOption, UserSettings};
use crate::tasks::models::*;
use ts_rs::TS;

#[cfg(test)]
mod ts_export_tests {
    use super::*;

    #[test]
    fn export_settings_types() {
        UserSettings::export().unwrap();
        HudSizeOption::export().unwrap();
        HudDimensions::export().unwrap();
    }

    #[test] 
    fn export_task_types() {
        Task::export().unwrap();
        TaskStep::export().unwrap();
        TaskProgress::export().unwrap();
        TaskStatus::export().unwrap();
        StepStatus::export().unwrap();
        TaskFrequency::export().unwrap();
        CreateTaskRequest::export().unwrap();
        CreateTaskStepRequest::export().unwrap();
        UpdateTaskRequest::export().unwrap();
        TaskProgressUpdate::export().unwrap();
        StepUpdate::export().unwrap();
        TaskStatusCounts::export().unwrap();
        TaskDetectionResult::export().unwrap();
        CompletedStepDetection::export().unwrap();
        ScreenContext::export().unwrap();
        TaskWithSteps::export().unwrap();
    }
}
