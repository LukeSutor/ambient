# Cortical

Local‑first AI assistant that lives on your PC and helps automate everyday tasks.

## Overview

Cortical is a desktop app that runs AI models locally via llama.cpp, with an optional cloud fallback. Think of it as a lightweight “brain for your computer”: chat with it, give it context from your screen, and let it help with routine tasks. It runs efficiently in the background and prioritizes privacy and speed by default.

Built with Tauri (Rust + Next.js).

## Key features

- Local‑first inference with llama.cpp
  - Ships with a built‑in model downloader
- Floating chat window
  - Chat locally with Qwen3 1.7B
  - Optional GPT‑OSS‑20B and GPT-5 via OpenRouter
- Gemini-powered computer use
  - Let Gemini take control of your computer to complete tasks on your behalf
- Screen region context with OCR
  - Snipping‑tool‑style region capture
  - Extracts text with OCR and injects it into the conversation
- Runs in the background, designed to be helpful without getting in your way

## How it works

- The Tauri/Rust backend manages windowing, screen capture, OCR, and model orchestration
- llama.cpp runs locally; the app communicates with it through a local http server
- When you choose, OpenRouter can be used as the model provider
- All local artifacts (models, database, caches) never leave your computer

## Roadmap (vision)

- Proactive assistance: draft emails or messages based on detected screen context
- Scheduling: suggest/create calendar events from chats or on‑screen cues
- Rich automations: configurable actions triggered by screen activity
- Cross‑platform: macOS and Linux support

## Quick start (Windows)

Prerequisites
- Rust toolchain (MSVC)
- Node.js LTS and pnpm

Steps
1) Clone the repo and install dependencies
2) Create and fill in `app/src-tauri/.env` (see `.env.example`)
3) Start the app in development mode using `pnpm run tauri dev`

Windows PowerShell

```powershell
cd .\app
pnpm install
copy .\src-tauri\.env.example .\src-tauri\.env
# Edit .\src-tauri\.env to add your keys if needed
pnpm run tauri dev
```

## Configuration

- Local models via llama.cpp
  - A built‑in downloader fetches models
  - Default: Qwen3 1.7B
- Optional OpenRouter
  - Set `OPENROUTER_API_KEY` in `app/src-tauri/.env`

See `.env.example` for all supported variables.

## OCR and screen capture

- Region selection opens a snipping‑style overlay; the selected area is OCR’d and added to the chat context
- OCR is powered by the Rust crate [ocrs](https://github.com/robertknight/ocrs)

## Privacy

- Local‑first by design: inference and screen processing happen on your machine
- Optional cloud access through OpenRouter is opt‑in and key‑gated
- Artifacts (models, logs, OCR snippets) are stored locally

## Project structure

- `app/` – Tauri app with Next.js frontend and Rust backend
- `app/src-tauri/` – Tauri config, Rust commands, binaries, and environment
- `ml/` – experiments and training scripts
