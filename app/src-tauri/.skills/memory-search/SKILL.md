---
name: memory-search
description: Search through stored memories and past conversations. Use when the user references previous discussions, asks you to remember something, or when context from past interactions would help answer their question.
version: "1.1"
requires_auth: false
tools:
  - name: search_memories
    description: Search memories using semantic similarity or time range
    parameters:
      query:
        type: string
        description: The search query (optional if time range provided)
        required: false
      start_date:
        type: string
        description: Start date in ISO8601 format (e.g. 2026-02-05T00:00:00Z)
        required: false
      end_date:
        type: string
        description: End date in ISO8601 format (e.g. 2026-02-05T23:59:59Z)
        required: false
      limit:
        type: integer
        description: Maximum number of results
        required: false
        default: 10
      min_similarity:
        type: number
        description: Minimum similarity threshold (0-1)
        required: false
        default: 0.5
---

# Memory Search Skill

Search through stored memories from past conversations.

## When to Use
- User says "remember when..." or "recall..."
- User references past discussions or work
- User asks for a summary of a specific time period (e.g., "what did we do yesterday?")
- Context from previous interactions needed

## Guidelines
1. **Time Filtering**: Use the `Current Date` in your system prompt to calculate ISO8601 ranges for queries like "last week", "yesterday", or "two days ago".
2. **Hybrid Search**: Provide a `query` for semantic or keyword search.
3. **Log Retrieval**: If the user asks for a summary of a day without specific keywords, omit the `query` and provide `start_date` and `end_date` to get a chronological log of that period.
4. **Iterate**: Consider multiple search attempts if the first doesn't find relevant results.
5. **Context**: Present findings clearly in the context of the current conversation.