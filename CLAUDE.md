# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

TaskAware is a privacy-conscious, AI-powered desktop productivity assistant built with Tauri (Rust backend + Next.js frontend). It monitors screen activity locally, matches actions to tasks using on-device LLMs, and provides task tracking without sending sensitive data to the cloud.

## Architecture

### Core Components
- **Tauri App** (`app/`): Main desktop application with Next.js frontend and Rust backend
- **Backend** (`backend/`): Python scripts for LLM inference using SmolVLM and llama.cpp
- **ML Training** (`ml/smolvlm-training/`): Scripts for training custom vision-language models
- **Browser Extension** (`extension/`): Chrome extension for web-based activity tracking

### Key Modules (Rust)
- `auth/`: OAuth2 authentication (Google, Microsoft) with AWS Cognito integration
- `os_utils/`: Windows UI Automation for screen text extraction
- `integrations/`: Chromium workflow integration and server management
- `models/`: Local LLM handling (Qwen3) via mistralrs
- `vlm/`: Vision-language model integration for image analysis
- `events/`: Event-driven architecture for task tracking
- `db/`: SQLite database with vector embeddings (sqlite-vec)
- `scheduler/`: Task scheduling and automation

## Development Commands

### Frontend (Next.js)
```bash
cd app
pnpm dev           # Start development server with Turbopack
pnpm build         # Build for production
pnpm lint          # Run Biome linter and Next.js lint
```

### Tauri App
```bash
cd app
pnpm tauri dev     # Start Tauri development mode
pnpm tauri build   # Build desktop app
```

### Quick Start
```bash
./run.sh           # Shortcut to start Tauri dev mode
```

### Backend (Python)
```bash
cd backend
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate
pip install -r requirements.txt
python src/main.py <image_path> <prompt>  # Run SmolVLM inference
```

### ML Training
```bash
cd ml/smolvlm-training
pip install -r requirements.txt
python scripts/train.py          # Train custom model
python scripts/inference.py      # Test model inference
python scripts/generate_data.py  # Generate training data
```

## Key Technologies

- **Tauri 2.0**: Cross-platform desktop framework
- **Next.js 15**: React framework with Turbopack for fast development
- **Rust**: System-level backend with Windows UI Automation
- **mistralrs**: Local LLM inference (Qwen3 models)
- **SmolVLM**: Vision-language model for image understanding
- **SQLite + sqlite-vec**: Database with vector embeddings
- **AWS Cognito**: User authentication
- **llama.cpp**: Local LLM inference backend

## Authentication Flow

The app uses OAuth2 with Google/Microsoft providers, integrated with AWS Cognito for user management. Deep links handle OAuth callbacks. Auth tokens are stored securely using the keyring crate.

## Screen Monitoring

Windows UI Automation extracts text from active windows, which is processed by local LLMs to understand user activities and match them to predefined tasks without sending data externally.

## Database Schema

SQLite database stores:
- User sessions and auth tokens
- Task definitions and progress
- Screen capture metadata
- Vector embeddings for semantic search

## Event System

Event-driven architecture using Tauri's event system:
- Screen capture events
- Task completion events
- LLM inference events
- Scheduler events for automated task checking

## Testing

Currently no formal test suite - development focuses on manual testing and LLM evaluation scripts in the ML training directory.

## Linting & Code Quality

- **Frontend**: Biome for JavaScript/TypeScript linting
- **Rust**: Standard cargo fmt and clippy
- **Python**: No formal linting configured

## Environment Setup

Required environment variables:
- AWS Cognito configuration for authentication
- OAuth2 client credentials for Google/Microsoft
- Store in `.env` file in project root