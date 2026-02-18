---
name: automation-management
description: Create, list, and run background automation tasks. Use this to set up recurring tasks, scheduled actions, or event-driven automations. The user always confirms before an automation is created.
metadata:
  version: "2.0"
  requires_auth: false
  requires_google_auth: false
  requires_online: false
  tools:
    - name: list_automations
      description: List all automation tasks with their status, schedule, and last run info
      parameters: {}
    - name: create_automation
      description: "Propose a new automation task for the user to review and confirm. Opens the Create Automation form pre-filled with your suggestion — the user can adjust and click Create to save."
      parameters:
        name:
          type: string
          description: A short, descriptive name for the automation (e.g. "Daily Sales Summary")
          required: true
        description:
          type: string
          description: One-sentence explanation of what this automation does
          required: false
        task_type:
          type: string
          description: "'scheduled' for time-based automations, 'semantic' for screen/URL triggered automations"
          required: true
        prompt:
          type: string
          description: The instruction prompt the agent will execute when this automation runs. Be specific and action-oriented.
          required: true
        schedule_type:
          type: string
          description: "For scheduled tasks: 'interval' (every N minutes), 'daily' (once per day at a time), 'weekdays' (Mon-Fri at a time), or 'specific_days' (chosen days at a time)"
          required: false
        schedule_value:
          type: string
          description: "The schedule value. For interval: minutes as a number (e.g. '30'). For daily/weekdays: 24h time (e.g. '09:00'). For specific_days: pipe-separated days and time (e.g. 'monday,wednesday|09:00')."
          required: false
        trigger_type:
          type: string
          description: "For semantic tasks: 'screen_content' (fires when keywords appear on screen) or 'url_visit' (fires when a URL pattern is detected)"
          required: false
        trigger_config:
          type: string
          description: "JSON config for the trigger. For screen_content: {\"keywords\": [\"error\", \"alert\"]}. For url_visit: {\"url_patterns\": [\"github.com/pulls\"]}"
          required: false
    - name: run_automation
      description: Manually trigger an existing automation task to run immediately
      parameters:
        task_id:
          type: string
          description: The ID of the automation task to run (get IDs from list_automations)
          required: true
---

# Automation Management Skill

You can list automations, propose new ones for the user to confirm, and manually run existing ones.

## Important: User Confirms All Changes

When you call `create_automation`, it does **not** create the automation immediately. Instead, it opens the Create Automation form in the dashboard pre-filled with your proposed values. The user reviews, adjusts if needed, and clicks Create to confirm. Make sure to tell the user this in your response.

## Task Types

### Scheduled Tasks
Run on a time-based schedule:
- **interval**: Runs every N minutes. `schedule_value`: number of minutes (e.g. `"30"`)
- **daily**: Runs at a specific time every day. `schedule_value`: 24h time (e.g. `"09:00"`)
- **weekdays**: Runs at a specific time Monday–Friday. `schedule_value`: 24h time (e.g. `"08:30"`)
- **specific_days**: Runs on chosen days at a specific time. `schedule_value`: `"day1,day2|HH:MM"` (e.g. `"monday,wednesday,friday|09:00"`)

### Semantic / Trigger-Based Tasks
Triggered when events are detected on the user's screen:
- **screen_content**: Fires when specific keywords appear anywhere on screen. Config: `{"keywords": ["error", "alert", "failed"]}`
- **url_visit**: Fires when a URL pattern is detected visible on screen. Config: `{"url_patterns": ["github.com/pulls", "jira.com"]}`

## Writing Good Prompts
- Be specific and action-oriented: "Check the NVIDIA stock price and write a 3-sentence summary of daily performance" not "summarize stocks"
- The agent runs without user interaction, so the prompt should be self-contained
- Include any relevant context (what to look for, how to format the output, where to send results)

## Guidelines
- Always use `list_automations` first to check what already exists before proposing duplicates
- Set reasonable schedules — avoid creating automations that run too frequently (prefer 30+ minute intervals)
- For the `run_automation` tool: inform the user that you've triggered the run and they can see results in the Automations dashboard
