use crate::tasks::models::{CreateTaskRequest, CreateTaskStepRequest, TaskFrequency};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTemplate {
  pub name: String,
  pub description: Option<String>,
  pub category: String,
  pub priority: i32,
  pub frequency: TaskFrequency,
  pub steps: Vec<TaskStepTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStepTemplate {
  pub title: String,
  pub description: Option<String>,
}

impl TaskTemplate {
  pub fn to_create_request(&self) -> CreateTaskRequest {
    CreateTaskRequest {
      name: self.name.clone(),
      description: self.description.clone().unwrap_or_default(),
      category: Some(self.category.clone()),
      priority: self.priority,
      frequency: self.frequency.clone(),
      steps: self
        .steps
        .iter()
        .map(|step| step.to_create_request())
        .collect(),
    }
  }
}

impl TaskStepTemplate {
  pub fn to_create_request(&self) -> CreateTaskStepRequest {
    CreateTaskStepRequest {
      title: self.title.clone(),
      description: self.description.clone().unwrap_or_default(),
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
      frequency: TaskFrequency::Weekly, // Marketing campaigns are often weekly
      steps: vec![
        TaskStepTemplate {
          title: "Open Email Marketing Platform".to_string(),
          description: Some("Navigate to email marketing tool dashboard".to_string()),
        },
        TaskStepTemplate {
          title: "Create New Campaign".to_string(),
          description: Some("Start new email campaign creation".to_string()),
        },
        TaskStepTemplate {
          title: "Design Email Content".to_string(),
          description: Some("Create or select email template and add content".to_string()),
        },
        TaskStepTemplate {
          title: "Configure Campaign Settings".to_string(),
          description: Some("Set recipient list, send time, and campaign options".to_string()),
        },
        TaskStepTemplate {
          title: "Schedule or Send Campaign".to_string(),
          description: Some("Schedule the campaign for later or send immediately".to_string()),
        },
      ],
    },
    // Customer Support Ticket Resolution
    TaskTemplate {
      name: "Customer Support Ticket Resolution".to_string(),
      description: Some("Process and resolve a customer support ticket".to_string()),
      category: "Customer Support".to_string(),
      priority: 3,
      frequency: TaskFrequency::OneTime, // Support tickets are one-time
      steps: vec![
        TaskStepTemplate {
          title: "Open Support Ticket System".to_string(),
          description: Some("Access the customer support ticket dashboard".to_string()),
        },
        TaskStepTemplate {
          title: "Review Ticket Details".to_string(),
          description: Some("Read customer issue description and gather context".to_string()),
        },
        TaskStepTemplate {
          title: "Research Solution".to_string(),
          description: Some("Look up relevant documentation or knowledge base".to_string()),
        },
        TaskStepTemplate {
          title: "Respond to Customer".to_string(),
          description: Some("Write and send response with solution or next steps".to_string()),
        },
        TaskStepTemplate {
          title: "Update Ticket Status".to_string(),
          description: Some("Mark ticket as resolved, pending, or escalated".to_string()),
        },
      ],
    },
    // Social Media Content Creation
    TaskTemplate {
      name: "Social Media Content Creation".to_string(),
      description: Some("Create and schedule social media posts across platforms".to_string()),
      category: "Social Media".to_string(),
      priority: 2,
      frequency: TaskFrequency::Daily, // Social media content is often daily
      steps: vec![
        TaskStepTemplate {
          title: "Open Social Media Management Tool".to_string(),
          description: Some("Access social media scheduling platform".to_string()),
        },
        TaskStepTemplate {
          title: "Create Post Content".to_string(),
          description: Some("Write post text and select images or media".to_string()),
        },
        TaskStepTemplate {
          title: "Select Target Platforms".to_string(),
          description: Some("Choose which social platforms to post to".to_string()),
        },
        TaskStepTemplate {
          title: "Schedule Publication".to_string(),
          description: Some("Set date and time for post publication".to_string()),
        },
        TaskStepTemplate {
          title: "Confirm and Queue Post".to_string(),
          description: Some("Review and confirm the scheduled post".to_string()),
        },
      ],
    },
    // Sales Lead Follow-up
    TaskTemplate {
      name: "Sales Lead Follow-up".to_string(),
      description: Some("Follow up with a potential sales lead via email or call".to_string()),
      category: "Sales".to_string(),
      priority: 3,
      frequency: TaskFrequency::Custom(3), // Follow up every 3 days
      steps: vec![
        TaskStepTemplate {
          title: "Open CRM System".to_string(),
          description: Some("Access customer relationship management system".to_string()),
        },
        TaskStepTemplate {
          title: "Review Lead Information".to_string(),
          description: Some("Check lead details, previous interactions, and context".to_string()),
        },
        TaskStepTemplate {
          title: "Draft Follow-up Message".to_string(),
          description: Some(
            "Compose personalized follow-up email or prepare call notes".to_string(),
          ),
        },
        TaskStepTemplate {
          title: "Send Follow-up or Make Call".to_string(),
          description: Some("Send the follow-up email or place the phone call".to_string()),
        },
        TaskStepTemplate {
          title: "Log Interaction in CRM".to_string(),
          description: Some("Record the follow-up activity and update lead status".to_string()),
        },
      ],
    },
    // Document Review and Approval
    TaskTemplate {
      name: "Document Review and Approval".to_string(),
      description: Some("Review, edit, and approve a business document".to_string()),
      category: "Administration".to_string(),
      priority: 2,
      frequency: TaskFrequency::OneTime, // Document reviews are typically one-time
      steps: vec![
        TaskStepTemplate {
          title: "Open Document Management System".to_string(),
          description: Some("Access document repository or file sharing system".to_string()),
        },
        TaskStepTemplate {
          title: "Locate and Open Document".to_string(),
          description: Some("Find the document that needs review and open it".to_string()),
        },
        TaskStepTemplate {
          title: "Review Document Content".to_string(),
          description: Some("Read through document and identify areas needing changes".to_string()),
        },
        TaskStepTemplate {
          title: "Make Necessary Edits".to_string(),
          description: Some("Edit document content, add comments, or suggest changes".to_string()),
        },
        TaskStepTemplate {
          title: "Approve or Request Changes".to_string(),
          description: Some("Either approve the document or send back for revisions".to_string()),
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
