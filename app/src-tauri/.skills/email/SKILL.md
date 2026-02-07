---
name: email
description: Access the user's Gmail to read recent messages for context. Use when the user refers to information they received via email or asks for a summary of their recent inbox.
version: "1.0"
requires_auth: true
requires_google_auth: true
tools:
  - name: list_emails
    description: Retrieve a list of the user's most recent emails (metadata/previews only)
    parameters:
      limit:
        type: integer
        description: Number of emails to retrieve (default 5, max 20)
        required: false
  - name: get_email_details
    description: Retrieve the full content of a specific email by its ID
    parameters:
      id:
        type: string
        description: The unique message ID of the email to retrieve
        required: true
---

# Email Skill

Read recent Gmail messages for context.

## When to Use
- Answering questions about received emails
- Summarizing recent communications
- Finding specific information requested via email

## Guidelines
1. Only read the number of emails necessary for the context
2. Respect user privacy - do not expose email contents unless relevant to the task
3. Use the get_email_details tool when specific information from an email is required