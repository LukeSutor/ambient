# Ambient

Ambient is a local-first AI assistant that aims to have all the features of a cloud-based service like ChatGPT, Claude, or Gemini. It comes with an agentic chat runtime that includes web search, code execution, agentic memory, and Google Mail and Calendar integration.

## Overview

Ambient is a desktop app that runs AI models locally via llama.cpp, with an optional cloud fallback. Think of it as a lightweight “brain for your computer”: chat with it, give it context from your screen, and let it help with routine tasks. It runs efficiently in the background and prioritizes privacy and speed.

Built with Tauri (Rust + Next.js).

## Key features

- Local‑first inference with llama.cpp
  - Ships with a built‑in model downloader
- Agentic runtime built on top of the [skills](https://agentskills.io/home) specification
  - Web search using Tauri's webview window as a scraper
  - Sandboxed code execution using [rustpython](https://github.com/RustPython/RustPython)
  - Agentic memory for remembering past conversations
  - Google mail and calendar integration for work automation
- Gemini-powered computer use
  - Let Gemini take control of your computer to complete tasks on your behalf
- Screen region context with OCR
  - Snipping‑tool‑style region capture
  - Extracts text with [ocrs](https://github.com/robertknight/ocrs) and injects it into the conversation
- Runs in the background, designed to be helpful without getting in your way

## How it works

- The Tauri/Rust backend manages windowing, screen capture, OCR, and model orchestration
- llama.cpp runs locally; the app communicates with it through a local http server
- When you choose, Gemini can be used for more advanced capabilities
- Supabase for auth and profile data
- Cloudflare Workers for cloud LLM completions
- All local artifacts (models, database, caches) never leave your computer

## Roadmap (vision)

- Proactive assistance: draft emails or messages based on detected screen context
- Scheduling: suggest/create calendar events from chats or on‑screen cues
- Rich automations: configurable actions triggered by screen activity

## Quick start (Windows)

Prerequisites
- Rust toolchain (MSVC)
- Node.js LTS and pnpm

Steps
1) Clone the repo and install dependencies
2) Create and fill in `cloudflare/workers/llm-completions/.env` (see `.env.example`)
3) Start the cloudflare worker using `pnpm run dev`
4) Start the app in development mode using `pnpm run tauri dev`

## Configuration

- Local models via llama.cpp
  - A built‑in downloader fetches Qwen3VL-2B
- Optional Gemini 3 Flash and Pro through Cloudflare Worker

## Privacy

- Local‑first by design: inference and screen processing happen on your machine
- Optional Gemini access through Cloudflare is opt‑in
- Artifacts (models, logs, messages, attachments) are stored locally

## Project structure

- `app/` – Tauri app with Next.js frontend and Rust backend
- `app/src-tauri/` – Tauri config, Rust commands, binaries, and environment
- `cloudflare/` – Cloudflare worker backend
- `ml/` – experiments and training scripts
