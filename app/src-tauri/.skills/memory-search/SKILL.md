---
name: memory-search
description: Search through stored memories and past conversations. Use when the user references previous discussions, asks you to remember something, or when context from past interactions would help answer their question.
version: "1.0"
requires_auth: false
tools:
  - name: search_memories
    description: Search memories using semantic similarity
    parameters:
      query:
        type: string
        description: The search query
        required: true
      limit:
        type: integer
        description: Maximum number of results
        required: false
        default: 5
      min_similarity:
        type: number
        description: Minimum similarity threshold (0-1)
        required: false
        default: 0.7
---

# Memory Search Skill

Search through stored memories from past conversations.

## When to Use
- User says "remember when..."
- User references past discussions
- Context from previous interactions needed

## Guidelines
1. Search with relevant keywords from the user's query
2. Consider multiple search attempts if first doesn't find relevant results
3. Present findings in context of the current conversation