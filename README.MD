# Ambient

A local-first AI desktop assistant. Run models on your machine, bring your own API keys, or use built-in cloud models — all from a lightweight app that lives in your system tray.

Built with [Tauri](https://tauri.app) (Rust + Next.js).

## Features

**Local inference**
- Ships with [llama.cpp](https://github.com/ggerganov/llama.cpp) and a built-in model downloader (Qwen3VL-2B)
- GPU support through the Vulkan API
- Vision support understands images and screen captures out of the box

**Agentic runtime**
- Web search via Tauri WebView
- Sandboxed Python code execution via [RustPython](https://github.com/RustPython/RustPython)
- Agentic memory across conversations
- Google Mail and Calendar integration
- Headless browser-use automation via Tauri Webview

**Bring Your Own Key (BYOK)**
- Connect any OpenAI, Gemini, or Anthropic-compatible API
- API keys are stored encrypted in a local SQLite database and never leave your machine

**Screen context**
- Snipping-tool-style region capture
- Local OCR with [ocrs](https://github.com/robertknight/ocrs)
- Extracted text is injected directly into the conversation

**Privacy-first**
- All local data (models, conversations, attachments) stays on your machine
- Encrypted databases (SQLCipher)
- Cloud features are disabled by default and opt-in

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs) (MSVC toolchain on Windows)
- [Node.js](https://nodejs.org) LTS + [pnpm](https://pnpm.io)

### Development

```bash
# Install dependencies
cd app && pnpm install

# Start the Cloudflare Worker (needed for built-in cloud models)
cd cloudflare/workers/ambient-backend
pnpm install && pnpm run dev

# In a separate terminal — start the app
cd app && pnpm run tauri dev
```

### Build

```bash
cd app && pnpm run tauri build
```

## Architecture

```
app/                    Tauri application
├── src/                Next.js frontend (React 19, TypeScript, shadcn/ui)
├── src-tauri/          Rust backend
│   └── src/
│       ├── agents/     Chat + browser-use agentic runtimes
│       ├── models/     LLM providers, llama.cpp server, embeddings
│       ├── skills/     Tool registry and execution
│       ├── db/         SQLite database, migrations, CRUD
│       ├── auth/       Supabase auth, OAuth, token storage (OS keyring + AES)
│       └── settings/   User preferences
cloudflare/             Cloudflare Worker backend (cloud LLM proxy, rate limiting)
```

## How It Works

- **Local models** run via llama.cpp as a background process — the app talks to it over a local HTTP server
- **BYOK models** connect directly to the provider's API using your key
- **Built-in cloud models** (Gemini) are proxied through a Cloudflare Worker
- **Supabase** handles authentication and user profiles
- **All local artifacts** (models, databases, caches, attachments) never leave your computer
