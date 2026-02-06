---
name: computer-use
description: Control the computer to perform actions on behalf of the user. Use when the user asks you to do something on their computer that requires mouse/keyboard interaction, like browsing websites, filling forms, or operating applications.
version: "1.0"
requires_auth: false
tools:
  - name: start_computer_use
    description: Start a computer use session with a specific goal
    parameters:
      goal:
        type: string
        description: The goal to accomplish
        required: true
---

# Computer Use Skill

Control the computer to perform tasks for the user.

## When to Use
- User asks to do something on their computer
- Tasks requiring mouse/keyboard interaction
- Automating multi-step workflows

## Guidelines
1. Clearly understand the goal before starting
2. Ask for confirmation for sensitive actions
3. Provide status updates during execution