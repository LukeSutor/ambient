use serde::{Deserialize, Serialize};
use crate::tasks::models::{CreateTaskRequest, CreateTaskStepRequest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTemplate {
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub priority: i32,
    pub estimated_duration: Option<i32>,
    pub steps: Vec<TaskStepTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStepTemplate {
    pub title: String,
    pub description: Option<String>,
    pub completion_criteria: String,
    pub application_context: Option<String>,
}

impl TaskTemplate {
    pub fn to_create_request(&self) -> CreateTaskRequest {
        CreateTaskRequest {
            name: self.name.clone(),
            description: self.description.clone(),
            category: Some(self.category.clone()),
            priority: self.priority,
            estimated_duration: self.estimated_duration,
            steps: self.steps.iter().map(|step| step.to_create_request()).collect(),
        }
    }
}

impl TaskStepTemplate {
    pub fn to_create_request(&self) -> CreateTaskStepRequest {
        CreateTaskStepRequest {
            title: self.title.clone(),
            description: self.description.clone(),
            completion_criteria: self.completion_criteria.clone(),
            application_context: self.application_context.clone(),
        }
    }
}

/// Returns predefined task templates for common business processes
pub fn get_built_in_templates() -> Vec<TaskTemplate> {
    vec![
        // Email Marketing Campaign
        TaskTemplate {
            name: "Email Marketing Campaign Setup".to_string(),
            description: Some("Create and schedule a marketing email campaign".to_string()),
            category: "Marketing".to_string(),
            priority: 2,
            estimated_duration: Some(45),
            steps: vec![
                TaskStepTemplate {
                    title: "Open Email Marketing Platform".to_string(),
                    description: Some("Navigate to email marketing tool dashboard".to_string()),
                    completion_criteria: "Email marketing platform dashboard is visible with main navigation elements like 'Campaigns', 'Templates', or 'Create Campaign'".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
                TaskStepTemplate {
                    title: "Create New Campaign".to_string(),
                    description: Some("Start new email campaign creation".to_string()),
                    completion_criteria: "Campaign creation form is open with fields for campaign name, subject line, or template selection".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
                TaskStepTemplate {
                    title: "Design Email Content".to_string(),
                    description: Some("Create or select email template and add content".to_string()),
                    completion_criteria: "Email editor is open with content being added, template selected, or email preview visible".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
                TaskStepTemplate {
                    title: "Configure Campaign Settings".to_string(),
                    description: Some("Set recipient list, send time, and campaign options".to_string()),
                    completion_criteria: "Campaign settings page is visible with recipient lists, scheduling options, or send configurations".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
                TaskStepTemplate {
                    title: "Schedule or Send Campaign".to_string(),
                    description: Some("Schedule the campaign for later or send immediately".to_string()),
                    completion_criteria: "Campaign confirmation page showing 'scheduled', 'sent', or success message is visible".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
            ],
        },

        // Customer Support Ticket Resolution
        TaskTemplate {
            name: "Customer Support Ticket Resolution".to_string(),
            description: Some("Process and resolve a customer support ticket".to_string()),
            category: "Customer Support".to_string(),
            priority: 3,
            estimated_duration: Some(30),
            steps: vec![
                TaskStepTemplate {
                    title: "Open Support Ticket System".to_string(),
                    description: Some("Access the customer support ticket dashboard".to_string()),
                    completion_criteria: "Support ticket system dashboard is visible with ticket queue, search, or ticket list".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
                TaskStepTemplate {
                    title: "Review Ticket Details".to_string(),
                    description: Some("Read customer issue description and gather context".to_string()),
                    completion_criteria: "Individual ticket view is open showing customer details, issue description, or ticket history".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
                TaskStepTemplate {
                    title: "Research Solution".to_string(),
                    description: Some("Look up relevant documentation or knowledge base".to_string()),
                    completion_criteria: "Knowledge base, documentation, or help articles are open and being referenced".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
                TaskStepTemplate {
                    title: "Respond to Customer".to_string(),
                    description: Some("Write and send response with solution or next steps".to_string()),
                    completion_criteria: "Ticket response form is open with message being composed or response confirmation visible".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
                TaskStepTemplate {
                    title: "Update Ticket Status".to_string(),
                    description: Some("Mark ticket as resolved, pending, or escalated".to_string()),
                    completion_criteria: "Ticket status is updated showing 'resolved', 'pending customer', 'escalated', or similar status".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
            ],
        },

        // Social Media Content Creation
        TaskTemplate {
            name: "Social Media Content Creation".to_string(),
            description: Some("Create and schedule social media posts across platforms".to_string()),
            category: "Social Media".to_string(),
            priority: 2,
            estimated_duration: Some(25),
            steps: vec![
                TaskStepTemplate {
                    title: "Open Social Media Management Tool".to_string(),
                    description: Some("Access social media scheduling platform".to_string()),
                    completion_criteria: "Social media management dashboard is visible with post creation options or content calendar".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
                TaskStepTemplate {
                    title: "Create Post Content".to_string(),
                    description: Some("Write post text and select images or media".to_string()),
                    completion_criteria: "Post creation form is open with text being written, images uploaded, or media being added".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
                TaskStepTemplate {
                    title: "Select Target Platforms".to_string(),
                    description: Some("Choose which social platforms to post to".to_string()),
                    completion_criteria: "Platform selection interface is visible with checkboxes for different social networks or platform-specific customization".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
                TaskStepTemplate {
                    title: "Schedule Publication".to_string(),
                    description: Some("Set date and time for post publication".to_string()),
                    completion_criteria: "Scheduling interface is open with date/time picker or calendar view for setting publication time".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
                TaskStepTemplate {
                    title: "Confirm and Queue Post".to_string(),
                    description: Some("Review and confirm the scheduled post".to_string()),
                    completion_criteria: "Post confirmation or queue view showing the scheduled post with success message".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
            ],
        },

        // Sales Lead Follow-up
        TaskTemplate {
            name: "Sales Lead Follow-up".to_string(),
            description: Some("Follow up with a potential sales lead via email or call".to_string()),
            category: "Sales".to_string(),
            priority: 3,
            estimated_duration: Some(20),
            steps: vec![
                TaskStepTemplate {
                    title: "Open CRM System".to_string(),
                    description: Some("Access customer relationship management system".to_string()),
                    completion_criteria: "CRM dashboard is visible with lead management, contact lists, or sales pipeline".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
                TaskStepTemplate {
                    title: "Review Lead Information".to_string(),
                    description: Some("Check lead details, previous interactions, and context".to_string()),
                    completion_criteria: "Lead profile or contact details are open showing interaction history, contact info, or lead status".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
                TaskStepTemplate {
                    title: "Draft Follow-up Message".to_string(),
                    description: Some("Compose personalized follow-up email or prepare call notes".to_string()),
                    completion_criteria: "Email composition window is open with message being written or call preparation notes are visible".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
                TaskStepTemplate {
                    title: "Send Follow-up or Make Call".to_string(),
                    description: Some("Send the follow-up email or place the phone call".to_string()),
                    completion_criteria: "Email sent confirmation is visible or phone dialer is active with call in progress".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
                TaskStepTemplate {
                    title: "Log Interaction in CRM".to_string(),
                    description: Some("Record the follow-up activity and update lead status".to_string()),
                    completion_criteria: "CRM activity log is open with new interaction being recorded or lead status being updated".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
            ],
        },

        // Document Review and Approval
        TaskTemplate {
            name: "Document Review and Approval".to_string(),
            description: Some("Review, edit, and approve a business document".to_string()),
            category: "Administration".to_string(),
            priority: 2,
            estimated_duration: Some(35),
            steps: vec![
                TaskStepTemplate {
                    title: "Open Document Management System".to_string(),
                    description: Some("Access document repository or file sharing system".to_string()),
                    completion_criteria: "Document management interface is visible with file listings, folders, or document search".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
                TaskStepTemplate {
                    title: "Locate and Open Document".to_string(),
                    description: Some("Find the document that needs review and open it".to_string()),
                    completion_criteria: "Document is open for viewing or editing in document editor or viewer".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
                TaskStepTemplate {
                    title: "Review Document Content".to_string(),
                    description: Some("Read through document and identify areas needing changes".to_string()),
                    completion_criteria: "Document content is being reviewed with comments being added, highlights made, or revision suggestions visible".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
                TaskStepTemplate {
                    title: "Make Necessary Edits".to_string(),
                    description: Some("Edit document content, add comments, or suggest changes".to_string()),
                    completion_criteria: "Document is in edit mode with changes being made, comments added, or track changes enabled".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
                TaskStepTemplate {
                    title: "Approve or Request Changes".to_string(),
                    description: Some("Either approve the document or send back for revisions".to_string()),
                    completion_criteria: "Approval workflow interface is visible with approval buttons, rejection options, or workflow status updated".to_string(),
                    application_context: Some("Chrome".to_string()),
                },
            ],
        },
    ]
}

/// Returns template by name
pub fn get_template_by_name(name: &str) -> Option<TaskTemplate> {
    get_built_in_templates()
        .into_iter()
        .find(|template| template.name == name)
}

/// Returns all available template names
pub fn get_template_names() -> Vec<String> {
    get_built_in_templates()
        .into_iter()
        .map(|template| template.name)
        .collect()
}

/// Returns templates filtered by category
pub fn get_templates_by_category(category: &str) -> Vec<TaskTemplate> {
    get_built_in_templates()
        .into_iter()
        .filter(|template| template.category == category)
        .collect()
}

/// Returns all available categories
pub fn get_categories() -> Vec<String> {
    let mut categories: Vec<String> = get_built_in_templates()
        .into_iter()
        .map(|template| template.category)
        .collect();
    categories.sort();
    categories.dedup();
    categories
}
