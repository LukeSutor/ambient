---
name: calendar
description: Access the user's Google Calendar to view schedules and create events. Use when the user asks about their schedule, meetings, or wants to set reminders and appointments.
version: "1.0"
requires_auth: true
requires_google_auth: true
tools:
  - name: list_events
    description: List events from the user's primary calendar
    parameters:
      start:
        type: string
        description: ISO 8601 start time (e.g., '2024-05-01T00:00:00Z')
        required: false
      end:
        type: string
        description: ISO 8601 end time (e.g., '2024-05-02T00:00:00Z')
        required: false
  - name: create_event
    description: Create a new event in the user's primary calendar
    parameters:
      title:
        type: string
        description: The event title
        required: true
      start_time:
        type: string
        description: ISO 8601 start time
        required: true
      end_time:
        type: string
        description: ISO 8601 end time
        required: false
      description:
        type: string
        description: Event description
        required: false
---

# Calendar Skill

Manage the user's Google Calendar events.

## When to Use
- Checking availability
- Listing upcoming meetings or events
- Scheduling new appointments
- Setting placeholders in the calendar

## Guidelines
1. Always confirm details if creating an event from ambiguous text
2. Default to 'today' and 'tomorrow' if no range specified for list_events
3. Format event times clearly for the user
