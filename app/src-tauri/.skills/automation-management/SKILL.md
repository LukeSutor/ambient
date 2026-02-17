---
name: automation-management
description: Create, manage, and control background automation tasks. Use this to set up recurring tasks, scheduled actions, or event-driven automations.
metadata:
  version: "1.0"
  requires_auth: false
  requires_google_auth: false
  requires_online: false
  tools:
    - name: list_automations
      description: List all automation tasks with their status and schedule
      parameters: {}
    - name: create_automation
      description: Create a new automation task
      parameters:
        name:
          type: string
          description: A short, descriptive name for the automation
          required: true
        description:
          type: string
          description: What this automation does
          required: false
        task_type:
          type: string
          description: "Type of automation: 'scheduled' (time-based) or 'semantic' (trigger-based)"
          required: true
        prompt:
          type: string
          description: The instruction prompt that the agent will execute when this automation runs
          required: true
        schedule_type:
          type: string
          description: "For scheduled tasks: 'interval' (every N minutes), 'daily' (at HH:MM), 'weekly' (DAY,HH:MM), or 'once' (ISO 8601 datetime)"
          required: false
        schedule_value:
          type: string
          description: "The schedule value. Examples: '30' (every 30 min), '09:00' (daily at 9am), 'monday,09:00' (weekly), '2025-01-01T00:00:00Z' (once)"
          required: false
        trigger_type:
          type: string
          description: "For semantic tasks: 'screen_content', 'url_visit', or 'app_focus'"
          required: false
        trigger_config:
          type: string
          description: "JSON config for the trigger. Example: {\"keywords\": [\"error\", \"alert\"]}"
          required: false
    - name: toggle_automation
      description: Enable or disable an existing automation task
      parameters:
        task_id:
          type: string
          description: The ID of the automation task to toggle
          required: true
        enabled:
          type: boolean
          description: Whether to enable (true) or disable (false) the automation
          required: true
    - name: delete_automation
      description: Delete an automation task permanently
      parameters:
        task_id:
          type: string
          description: The ID of the automation task to delete
          required: true
    - name: run_automation
      description: Manually trigger an automation task to run immediately
      parameters:
        task_id:
          type: string
          description: The ID of the automation task to run
          required: true
---

# Automation Management Skill

You can create, list, toggle, delete, and manually run automation tasks.

## Task Types

### Scheduled Tasks
Run on a time-based schedule:
- **interval**: Runs every N minutes (e.g., `schedule_value: "30"` = every 30 minutes)
- **daily**: Runs at a specific time each day (e.g., `schedule_value: "09:00"`)
- **weekly**: Runs on a specific day and time (e.g., `schedule_value: "monday,09:00"`)
- **once**: Runs once at a specific datetime (e.g., `schedule_value: "2025-07-01T09:00:00Z"`)

### Semantic Tasks
Triggered by events detected on screen:
- **screen_content**: Fires when specific keywords appear on screen. Config: `{"keywords": ["error", "alert"]}`
- **url_visit**: Fires when a URL pattern is detected. Config: `{"url_patterns": ["github.com"]}`
- **app_focus**: Fires when a specific application is in focus. Config: `{"app_name": "Visual Studio Code"}`

## Guidelines
- Always give automations clear, descriptive names
- Set reasonable schedules — don't create automations that run too frequently
- Write clear prompt instructions that the agent can execute independently
- Use `list_automations` first to check what already exists before creating duplicates
