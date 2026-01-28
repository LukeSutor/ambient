---
name: web-search
description: Search the web for current information, news, documentation, and answers. Use when the user asks about recent events, needs factual information you're uncertain about, or requests real-time data like weather, stocks, or sports scores.
version: "1.0"
requires_auth: false
tools:
  - name: search_web
    description: Perform a web search and return relevant results
    parameters:
      query:
        type: string
        description: The search query
        required: true
      num_results:
        type: integer
        description: Number of results to return (1-10)
        required: false
        default: 5
  - name: fetch_webpage
    description: Fetch and extract main content from a specific URL
    parameters:
      url:
        type: string
        description: The URL to fetch
        required: true
---

# Web Search Skill

Search the internet for information to answer user questions.

## When to Use
- Questions about current events or recent news
- Facts you're uncertain about
- Real-time data (weather, stocks, sports)
- Technical documentation or references

## Guidelines
1. Use clear, specific search queries
2. Search first, then fetch specific pages if needed
3. If the search doesn't return what you need, fetch_webpage to get more information
4. Always cite sources