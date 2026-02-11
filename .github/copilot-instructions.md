# Ambient AI Assistant - Project Guidelines

## Project Overview

Ambient is a local-first AI desktop assistant built with **Tauri 2.x** (Rust + Next.js 15). It features an agentic runtime with tool-calling capabilities, local inference via llama.cpp, optional cloud fallback to Gemini, and browser-use capabilities. The architecture prioritizes privacy, extensibility, and reliability.

**Tech Stack:**
- Backend: Rust with Tauri 2.x
- Frontend: Next.js 15 (SSG mode), React 19, TypeScript
- UI: shadcn/ui (Radix primitives) + Tailwind CSS v4
- Local LLM: llama.cpp server (ships with Qwen3VL-2B)
- Cloud LLM: Gemini via Cloudflare Worker
- Database: SQLite with rusqlite + sqlite-vec for embeddings
- State: React Context + useReducer pattern

## Code Style

### TypeScript/React
- Components: `PascalCase.tsx` in [src/components/](app/src/components/)
- Use `"use client"` directive only when hooks/interactivity needed (see [layout.tsx](app/src/app/layout.tsx))
- Import order: React/Next → third-party → @/components → @/types → @/lib
- Component structure: types → component → hooks → effects → handlers → render
- File naming: `kebab-case.tsx` for files, `PascalCase` for component names
- shadcn/ui pattern: Use `cn()` utility for conditional classes (see [app-sidebar.tsx](app/src/components/app-sidebar.tsx))

### Rust
- Naming: `snake_case` modules/functions, `PascalCase` structs/enums, `SCREAMING_SNAKE_CASE` constants
- Error handling: Custom error enums with `thiserror`, convert to `String` for Tauri commands
- Logging: Prefix with module name: `log::info!("[module_name] Context message")`
- Module structure: `mod.rs` (re-exports), `types.rs`, `commands.rs`, service files (see [agents/](app/src-tauri/src/agents/))

## Architecture

### Frontend State Management

**Provider Hierarchy** ([AppProvider.tsx](app/src/lib/providers/AppProvider.tsx)):
```
SettingsProvider → RoleAccessProvider → SetupProvider → WindowsProvider → ConversationProvider
```

**Pattern:** Context + useReducer for all state management
- Define state interface + discriminated union actions
- Create reducer with switch statement
- Provider sets up event listeners for Tauri events via `listen()`
- Export custom hook that validates context exists

**Key Providers:**
- [ConversationProvider](app/src/lib/conversations/ConversationProvider.tsx): Chat messages, streaming, attachments
- [SettingsProvider](app/src/lib/settings/SettingsProvider.tsx): User settings, HUD dimensions
- [WindowsProvider](app/src/lib/windows/WindowsProvider.tsx): Window expand/collapse state
- [RoleAccessProvider](app/src/lib/role-access/RoleAccessProvider.tsx): Auth state, user info
- [SetupProvider](app/src/lib/setup/SetupProvider.tsx): Model download progress

### Backend Module Organization

**Top-level modules** ([lib.rs](app/src-tauri/src/lib.rs)):
- `auth/`: OAuth flow, split token storage (refresh tokens in OS keyring, session tokens AES-encrypted in store.json)
- `db/`: SQLite with migrations, conversations, messages, memory, token usage
- `agents/`: Chat runtime + browser-use runtime
- `models/`: LLM client (local/cloud providers), llama.cpp server, embedding, OCR
- `skills/`: Registry, executor, builtin skills (web-search, code-execution, memory)
- `events/`: Global emitter, typed event payloads
- `settings/`: User preferences, agent runtime config
- `windows/`: Window management, screen capture

**Module Pattern:**
```
module/
├── mod.rs          # Re-exports
├── types.rs        # Structs/enums
├── commands.rs     # Tauri commands
└── service.rs      # Business logic
```

### Agentic Chat Runtime

**Location:** [agents/chat/runtime.rs](app/src-tauri/src/agents/chat/runtime.rs)

**Loop Architecture:**
1. Get conversation history (context-limited based on local vs cloud)
2. Build `LlmRequest` with system prompt, messages, available tools
3. Generate response via provider (local or cloud)
4. If text response → save message and return
5. If tool calls → execute in parallel, save results as messages, loop continues
6. Repeat until text response or max iterations reached
7. Check cancellation signal (`Arc<AtomicBool>`) on each iteration

**Progressive Skill Disclosure:**
- Skills start inactive to reduce context size
- Send skill summaries (name + description) initially
- Model calls `activate_skill(skill_name)` to load full tools
- Activated skills persist for conversation lifetime
- See [skills/registry.rs](app/src-tauri/src/skills/registry.rs)

**Configuration:** [AgentRuntimeConfig](app/src/types/settings.ts) controls context limits (local: 5 messages, cloud: 15), max iterations, tool calls per turn, thinking mode

## Build and Test

### Development Setup
```bash
# 1. Install dependencies
cd app
pnpm install

# 2. Start Cloudflare Worker (required for cloud models)
cd cloudflare/workers/llm-completions
pnpm run dev

# 3. Start Tauri dev (separate terminal)
cd app
pnpm run tauri dev
```

### Production Build
```bash
cd app
pnpm run tauri build  # Build Tauri app with bundled frontend
```

### Checking
```bash
cd app/src-tauri
cargo check
```
**Always cargo check before finishing a feature.**

### Key Scripts
- `pnpm run lint`: Biome + ESLint checks
- `pnpm run tauri dev`: Development mode with hot reload
- `pnpm run tauri build`: Platform-specific installers
- `cargo check`: Rust compilation check

### Configuration Files
- [next.config.ts](app/next.config.ts): `output: "export"`, `distDir: "dist"`, static mode for Tauri
- [tauri.conf.json](app/src-tauri/tauri.conf.json): Base config
- Platform overrides: `tauri.windows.conf.json`, `tauri.macos.conf.json`, `tauri.linux.conf.json`

## Project Conventions

### Type Generation (Rust → TypeScript)
**NEVER hand-edit types in [src/types/](app/src/types/)**. They are auto-generated from Rust structs.

To add/modify types:
1. Edit Rust struct in [src-tauri/src/](app/src-tauri/src/)
2. Add `#[derive(TS)]` and `#[ts(export)]` attributes
3. Test project (`cargo test` inside of `app/src-tauri`) → types regenerate automatically
4. Import from `@/types` in frontend

Example:
```rust
#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MyType {
    pub field: String,
}
```

### Tauri IPC Patterns

**Invoking Commands** (Frontend → Backend):
```typescript
import { invoke } from "@tauri-apps/api/core";

const result = await invoke<ReturnType>("command_name", {
  paramName: value,
});
```

**Listening to Events** (Backend → Frontend):
```typescript
import { listen } from "@tauri-apps/api/event";

useEffect(() => {
  const setup = async () => {
    const unlisten = await listen<PayloadType>("event_name", (event) => {
      // Handle event.payload
    });
    return unlisten;
  };
  
  const cleanup = setup();
  return () => cleanup.then(fn => fn());
}, []);
```

**Emitting Events** (Frontend → Backend):
```typescript
import { emit } from "@tauri-apps/api/event";

await emit("event_name", { payload: "data" });
```

### Database Patterns

**Schema:** Located in [db/core.rs](app/src-tauri/src/db/core.rs) migrations

**Key Tables:**
- `conversations`: Conversation metadata
- `conversation_messages`: Messages with `message_type` (text/tool_calls/tool_results) and structured `metadata` JSON
- `attachments`: File attachments per message
- `memory_entries` + `memory_entries_vec`: Extracted facts with embeddings
- `tool_calls`: Tool execution audit log
- `conversation_skills`: Activated skills per conversation
- `token_usage`: LLM usage tracking (prompt_tokens, completion_tokens, timestamp)

**Access Pattern:**
```rust
let state = app_handle.state::<DbState>();
let conn = state.0.lock().unwrap();
let conn = conn.as_ref().ok_or("DB not initialized")?;
// Use conn...
```

**Message Storage:** Use structured metadata for tool calls/results, not content field:
```rust
pub enum MessageMetadata {
    ToolCall { call_id, skill_name, tool_name, arguments, ... },
    ToolResult { call_id, success, result, error, ... },
}
```

### LLM Provider Pattern

**Unified Interface:**
- All providers implement `LlmProvider` trait
- Use `LlmRequest`/`LlmResponse` types (see [models/llm/client.rs](app/src-tauri/src/models/llm/client.rs))
- Providers handle translation to OpenAI/Gemini formats internally

**Providers:**
- `LocalProvider`: Communicates with llama.cpp server (OpenAI-compatible API)
- `CloudflareProvider`: Proxies to Gemini via Cloudflare Worker

**Streaming:** Use `Arc<AtomicBool>` for cancellation signal, check in tight loops

### Skills System

**Location:** [src-tauri/src/skills/](app/src-tauri/src/skills/)

**Skill Definition:** YAML frontmatter + Markdown instructions in `.skills/*/SKILL.md`
```markdown
---
name: skill-name
description: Brief description
version: 1.0
requires_auth: false
tools:
  - name: tool_name
    description: What it does
    parameters:
      param_name:
        type: string
        description: Parameter description
        required: true
---

# Skill Instructions

Detailed instructions for the AI agent...
```

**Builtin Skills:**
- [web_search.rs](app/src-tauri/src/skills/builtin/web_search.rs): Uses Tauri WebView to bypass bot detection
- [code_execution.rs](app/src-tauri/src/skills/builtin/code_execution.rs): RustPython in isolated subprocess (crash-safe)
- `memory_search`: Vector similarity search on past conversations

**Tool Execution:** Always parallel with `futures::join_all`, handles errors per tool

## Integration Points

### Cloudflare Worker
**Location:** [cloudflare/workers/ambient-backend/](cloudflare/workers/ambient-backend/)

**Purpose:** Proxy for Gemini API (avoids exposing API keys) and backend for refreshing Google OAuth tokens (holds `GOOGLE_CLIENT_SECRET`).

**LLM Completion Request:**
```typescript
POST /
Authorization: Bearer <supabase_access_token>

{
  modelType: "fast" | "pro",
  content: [{ role, parts }],  // Gemini format
  stream: boolean,
  systemPrompt?: string,
  jsonSchema?: object,
  tools?: any,
}
```

**Google Token Refresh:**
```typescript
POST /v1/auth/google/refresh
Authorization: Bearer <supabase_access_token>

{ "refresh_token": "<google_refresh_token>" }
```
The worker verifies the Supabase token via `supabase.auth.getUser()`, then POSTs to `https://oauth2.googleapis.com/token` with the app's `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET` (stored as Cloudflare Worker secrets). Returns the new Google access token (and optionally a rotated refresh token) directly to the client. The client never sees the client secret.

**Model Mapping:**
- `fast`: gemini-3-flash-preview
- `pro`: gemini-3-pro-preview

### llama.cpp Server
**Management:** [models/llm/server.rs](app/src-tauri/src/models/llm/server.rs)
- Spawns as child process on app startup
- Random port selection with health checks
- Auto-restart on crash
- Ships with Qwen3VL-2B (text + vision model)
- Binaries in [src-tauri/binaries/](app/src-tauri/binaries/) per platform

### Authentication
**Flow:** OAuth via Supabase ([auth/](app/src-tauri/src/auth/))
1. Open Supabase auth URL with Google OAuth scopes in browser
2. Browser redirects to `ambient://oauth/callback#access_token=...&refresh_token=...` deep link
3. Extract tokens from URL fragment, validate by fetching user profile
4. Store session using split-storage architecture (see below)
5. Auto-refresh on expiry (Supabase tokens locally, Google tokens via Cloudflare Worker)

**Token Storage Architecture** ([storage.rs](app/src-tauri/src/auth/storage.rs)):

Tokens are split across two locations based on sensitivity and lifetime:

| Token | Location | Protection | Rationale |
|-------|----------|------------|----------|
| Supabase refresh token | OS keyring (`keyring` crate) | OS-level encryption | Long-lived, high-value — most secure location |
| Google refresh token | OS keyring (`keyring` crate) | OS-level encryption | Long-lived, high-value — never touches disk |
| AES encryption key | OS keyring (`keyring` crate) | OS-level encryption | Protects session tokens in store.json |
| Supabase access token | `store.json` (AES-256-GCM) | Encrypted with keyring key | Short-lived — fast access without keyring I/O |
| Google access token | `store.json` (AES-256-GCM) | Encrypted with keyring key | Short-lived — fast access without keyring I/O |
| Session metadata (user, expiry) | `store.json` (plaintext) | None (non-sensitive) | Quick reads for UI state |

Keyring service name: `"ambient"`. Entry names: `encryption_key`, `supabase_refresh_token`, `google_refresh_token`.

**Key functions in `storage.rs`:**
- `store_session()`: Splits a `Session` — refresh tokens → keyring, access tokens → AES-encrypt → store.json
- `retrieve_auth_state()`: Reads store.json + keyring, decrypts, reconstructs full `StoredAuthState`
- `get_refresh_token()` / `get_google_refresh_token()`: Read directly from keyring (no decryption needed)
- `get_access_token()` / `get_provider_token()`: Read from encrypted store.json
- `clear_auth_state()`: Clears both keyring entries and store.json

**Google Token Refresh Flow:**
1. `refresh_google_token()` acquires `REFRESH_MUTEX` to serialize refresh requests
2. If Supabase access token is expired, refreshes it first via `refresh_session_with_token()`
3. Reads Google refresh token from keyring (no decryption needed)
4. Sends refresh token to Cloudflare Worker (`/v1/auth/google/refresh`) which holds `GOOGLE_CLIENT_SECRET`
5. Worker calls Google's token endpoint and returns new access token
6. Stores updated session with new Google access token

## Security

### Sensitive Operations
- **Token storage:** Split architecture — refresh tokens in OS keyring (keychain), session tokens AES-256-GCM encrypted in store.json. If store.json is exfiltrated, refresh tokens remain safe in OS keyring.
- **Code execution:** Isolated subprocess to prevent app crashes
- **Web scraping:** Uses real browser engine to avoid exposing request patterns

### API Keys
- Never commit `.env` files
- Cloudflare Worker environment variables configured via Wrangler
- Supabase credentials in app only for auth, not in source

### Privacy-First
- All local artifacts (models, DB, caches) stay on device
- Cloud features (Gemini) are opt-in
- Screen capture and OCR happen locally (no external API calls)

## Machine Learning (ml/)
Contains training experiments and scripts for fine-tuning vision-language models (SmolVLM, InternVL). Not actively used in production. See [ml/smolvlm-training/](ml/smolvlm-training/) for data generation and training scripts.

---

## Quick Reference

**Adding a Tauri Command:**
1. Define `#[tauri::command]` function in `src-tauri/src/*/commands.rs`
2. Add to `invoke_handler!` macro in [lib.rs](app/src-tauri/src/lib.rs)
3. Call via `invoke("command_name", { params })` in frontend

**Adding an Event:**
1. Define event name constant in [events/types.rs](app/src-tauri/src/events/types.rs)
2. Define payload struct with `#[derive(TS)]` for frontend types
3. Emit via `events::emit(EVENT_NAME, payload)`
4. Listen in frontend via `listen<PayloadType>(EVENT_NAME, handler)`

**Adding a Provider State:**
1. Create reducer with state interface + action union in `src/lib/*/`
2. Export context hook that validates context exists
3. Wrap in [AppProvider.tsx](app/src/lib/providers/AppProvider.tsx) hierarchy
4. Use hook in components

**Adding a Skill:**
1. Create `SKILL.md` in `.skills/new-skill/` with YAML frontmatter
2. Implement handler in [skills/builtin/](app/src-tauri/src/skills/builtin/)
3. Register in [registry.rs](app/src-tauri/src/skills/registry.rs)
4. Handle tool execution in [executor.rs](app/src-tauri/src/skills/executor.rs)
