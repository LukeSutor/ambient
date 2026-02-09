---
name: calendar
description: Access the user's calendar to view schedules and create events. Use when the user asks about their schedule, meetings, or wants to set reminders and appointments.
metadata:
  version: "1.0"
  requires_auth: true
  requires_google_auth: true
  requires_online: true
  tools:
    - name: list_events
      description: List events from the user's primary calendar
      parameters:
        start:
          type: string
          description: RFC 3339 start time (e.g., '2024-05-01T00:00:00Z')
          required: false
        end:
          type: string
          description: RFC 3339 end time (e.g., '2024-05-02T00:00:00Z')
          required: false
        query:
          type: string
          description: Optional search query to filter events by text fields (eg. summary, location, attendees, etc.)
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
          description: RFC 3339 start time
          required: true
        end_time:
          type: string
          description: RFC 3339 end time
          required: false
        description:
          type: string
          description: Event description
          required: false
---

# Calendar Skill

Manage the user's calendar events.

## When to Use
- Checking availability
- Listing upcoming meetings or events
- Scheduling new events or reminders

## Guidelines
1. Always confirm details if creating an event from ambiguous text
2. Default to 'today' and 'tomorrow' if no range specified for list_events
3. Format event times clearly for the user
