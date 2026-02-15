# Ambient AI Assistant - Project Guidelines

## Project Overview

Ambient is a local-first AI desktop assistant built with **Tauri 2.x** (Rust + Next.js 15). It features an agentic runtime with tool-calling capabilities, local inference via llama.cpp, optional cloud fallback to Gemini, and browser-use capabilities. The architecture prioritizes privacy, extensibility, and reliability.

**Tech Stack:**
- Backend: Rust with Tauri 2.x
- Frontend: Next.js 15 (SSG mode), React 19, TypeScript
- UI: shadcn/ui (Radix primitives) + Tailwind CSS v4
- Local LLM: llama.cpp server (ships with Qwen3VL-2B)
- Cloud LLM: Gemini via Cloudflare Worker
- Database: SQLCipher (encrypted SQLite) with rusqlite + sqlite-vec for embeddings
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
SettingsProvider → RoleAccessProvider → ModelAccessProvider → SetupProvider → WindowsProvider → ConversationProvider
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
- [RoleAccessProvider](app/src/lib/role-access/RoleAccessProvider.tsx): Auth state, user info, Google auth status
- [SetupProvider](app/src/lib/setup/SetupProvider.tsx): Model download progress
- [ModelAccessProvider](app/src/lib/model-access/ModelAccessProvider.tsx): Cloud usage, model list, user tier — updates in real time via Tauri events (`cloud_usage_decremented`, `models_changed`, `auth_changed`)

### Backend Module Organization

**Top-level modules** ([lib.rs](app/src-tauri/src/lib.rs)):
- `auth/`: OAuth flow, split token storage (refresh tokens in OS keyring, session tokens AES-encrypted in per-user store.json)
- `db/`: Per-user encrypted SQLite (SQLCipher) with migrations, conversations, messages, memory, token usage
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
1. For cloud models: create a generation session via `/v1/usage/start-turn` (checks rate limit, increments usage once)
2. Get conversation history (context-limited based on local vs cloud)
3. Build `LlmRequest` with system prompt, messages, available tools, session token
4. Generate response via provider (local or cloud)
5. If text response → save message and return
6. If tool calls → execute in parallel, save results as messages, loop continues
7. Repeat until text response or max iterations reached
8. Check cancellation signal (`Arc<AtomicBool>`) on each iteration

**Session-Based Rate Limiting (Cloud Models):**
- Before the agentic loop, `create_generation_session(model_type)` calls the Cloudflare Worker's `/v1/usage/start-turn` endpoint
- This checks the user's daily limit, increments `model_usage`, and creates a `generation_sessions` row with a short-lived session token (10 min TTL, max 50 calls)
- The session token is attached to every `LlmRequest` in the loop via `.with_session_token()`
- The Cloudflare Worker validates the session on each `/v1/llm/generate` call — checks ownership, model match, expiry, and call count
- This means a multi-iteration agentic turn only counts as **one** usage against the daily limit
- Non-retryable errors: `rate_limit_exceeded`, `model_not_available`, `session_invalid`

### Browser-Use Runtime

**Location:** [agents/browser_use/runtime.rs](app/src-tauri/src/agents/browser_use/runtime.rs)

**Loop Architecture:**
1. Create persistent WebView
2. For cloud models: create a generation session (same as chat runtime)
3. LLM decides on an action (navigate, click, type, etc.)
4. Execute action and take a DOM snapshot
5. Return snapshot as tool result (attached to `MessageMetadata`)
6. Loop until `done` tool is called or max iterations reached
7. Strip old browser states from context to keep it lean (`strip_old_browser_states`)

**Browser Tools:** `navigate`, `click`, `type_text`, `select_option`, `scroll`, `go_back`, `wait`, `done`

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
cd cloudflare/workers/ambient-backend
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

### Per-User Data Isolation

All user data is stored in per-user profile directories under `{app_data}/profiles/{user_id}/`:

```
{app_data}/
├── models/                             # SHARED: downloaded ML model weights
├── profiles/
│   ├── {user_id_1}/
│   │   ├── database.sqlite             # Per-user encrypted DB (SQLCipher)
│   │   ├── store.json                  # Per-user settings + AES-encrypted auth tokens
│   │   └── attachments/{msg_id}/...    # Per-user file attachments
│   └── {user_id_2}/
│       └── ...
```

**No global store.json.** The active user is identified by `current_user_id` in the OS keyring, which bootstraps access to the per-user store on startup.

**SQLCipher Encryption:**
- Each user's DB is encrypted with a unique 32-byte random key
- Keys stored in OS keyring as `db_key_{user_id}` (no PBKDF2 overhead — raw key via `PRAGMA key = "x'{hex}'"`)    
- `rusqlite` feature: `bundled-sqlcipher-vendored-openssl` (bundles SQLCipher + OpenSSL)
- Verification on open: `SELECT count(*) FROM sqlite_master` to detect wrong key

**DB Lifecycle:**
1. App startup → reads `current_user_id` from keyring → if found, opens user DB via `initialize_user_database(app_handle, user_id)`
2. Login → frontend calls `invoke("open_user_database")` → closes old DB, opens new user's DB
3. Logout → `logout()` closes DB connection before clearing auth state
4. If no session on startup, DB stays `None` (initialized after login)

**Settings Per-User Store:**
- [settings/service.rs](app/src-tauri/src/settings/service.rs) reads `get_current_user_id()` from keyring to determine user ID
- Opens per-user store at `profiles/{user_id}/store.json` via `tauri_plugin_store`
- Returns `UserSettings::default()` silently when not logged in
- Errors on save when not logged in

**Attachments:** Stored at `profiles/{user_id}/attachments/{message_id}/filename` — path stored as relative in DB, resolved via `app_data.join(rel_path)`

**Shared (not per-user):** Model files, llama.cpp server, skill definitions, logs, transient screenshots

### Database Patterns

**Schema:** Located in [db/core.rs](app/src-tauri/src/db/core.rs) migrations

**Key Tables (SQLite/SQLCipher):**
- `conversations`: Conversation metadata
- `conversation_messages`: Messages with `message_type` (text/tool_calls/tool_results) and structured `metadata` JSON
- `attachments`: File attachments per message
- `models`: Registered LLM models with `id` (INTEGER PK), `model` (API identifier, not unique), `display_name`, `provider` (default 'unknown'), `is_internal` flag, BYOK fields (`api_url`, `api_key`, `request_format`). All lookups use `id`, not `model` text.
- `memory_entries` + `memory_entries_vec` + `memory_entries_fts`: Extracted facts with embeddings and full-text search
- `conversation_skills`: Activated skills per conversation
- `token_usage`: LLM usage tracking (prompt_tokens, completion_tokens, timestamp)

**Key Tables (Supabase - cloud):**
- `profiles`: User metadata (user_id, tier)
- `subscriptions`: Stripe subscription state
- `model_limits`: Per-tier daily limits keyed by model_type (e.g. "fast", "pro")
- `model_usage`: Daily usage counters per user per model_type (upserted on each turn)
- `generation_sessions`: Short-lived session tokens for per-turn rate limiting (10 min TTL, max 50 calls)

**Access Pattern:**
```rust
let state = app_handle.state::<DbState>();
let conn = state.0.lock().unwrap();
let conn = conn.as_ref().ok_or("DB not initialized")?;
// Use conn... (automatically points to current user's encrypted DB)
```

**Key Commands:**
- `open_user_database`: Reads auth state, closes existing connection, opens new user's encrypted DB
- `close_user_database`: Closes current DB connection (called on logout)
- `reset_database`: Deletes and recreates current user's DB

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
- Providers handle translation to OpenAI/Gemini/Anthropic formats internally

**Providers:**
- `LocalProvider`: Communicates with llama.cpp server (OpenAI-compatible API)
- `CloudflareProvider`: Proxies to Gemini via Cloudflare Worker
- `OpenAIProvider`: Direct OpenAI Chat Completions API (BYOK or `OPENAI_API_KEY` env var fallback)
- `GoogleProvider`: Direct Google Gemini REST API (BYOK or `GOOGLE_API_KEY` env var fallback, supports thinking/thought signatures)
- `AnthropicProvider`: Direct Anthropic Messages API (BYOK or `ANTHROPIC_API_KEY` env var fallback, event-based SSE streaming)

**BYOK Routing:**
`ResolvedModel` carries `id` (i64 PK), `model` (API identifier), `is_internal`, `api_url`, `api_key`, and `request_format` from the DB.
- Internal models: `!is_cloud` → LocalProvider, `is_cloud` → CloudflareProvider
- BYOK models: route by `request_format`: `"openai"` → OpenAIProvider, `"gemini"` → GoogleProvider, `"anthropic"` → AnthropicProvider
- Each provider resolves API key/URL from `ResolvedModel` first, falls back to env var
- `resolved_model.model` is the API identifier sent in requests
- `resolved_model.id` is used for token usage tracking and DB lookups

**BYOK Model Management:**
- `add_custom_model`: Validates, inserts BYOK model with `is_cloud=1, is_internal=0, daily_limit=NULL`, returns `i64` row id
- `update_custom_model`: Updates BYOK model fields by `id` (blocks internal models)
- `delete_custom_model`: Deletes BYOK model by `id`, auto-switches selection if deleted model was active
- `model` field = the API identifier sent in requests (e.g. `gpt-4o`, `claude-sonnet-4-20250514`). Not unique — multiple entries can share the same API model.
- `model_selection` in settings stores model `id` as string (e.g. `"1"`, `"4"`)
- Frontend form: react-hook-form + zod + shadcn Field in [model-dialog.tsx](app/src/components/secondary/settings/model-dialog.tsx)

**Model Display:**
- Provider images at `public/providers/{provider}.png`, local uses `public/logo.png`
- Token usage chart uses rotating color palette instead of per-model colors

**Streaming:** Use `Arc<AtomicBool>` for cancellation signal, check in tight loops

**Translation Layer** ([models/llm/providers/translation.rs](app/src-tauri/src/models/llm/providers/translation.rs)):
Bidirectional translation between internal message/tool types and three provider formats (OpenAI, Gemini, Anthropic).

Per-provider functions:
- `tools_to_{openai,gemini,anthropic}_format()`: Convert `ToolDefinition[]` to provider JSON
- `format_messages_for_{openai,gemini,anthropic}()`: Convert `Message[]` to provider message format
- `parse_{openai,gemini,anthropic}_tool_calls()`: Extract `ToolCall[]` from provider response JSON
- `has_tool_calls_{openai,gemini,anthropic}()`: Check if response contains tool calls
- `extract_text_{openai,gemini,anthropic}()`: Extract text content from response
- `extract_usage_{openai,gemini,anthropic}()`: Extract `TokenUsage { prompt_tokens, completion_tokens }` from response
- `extract_finish_reason_{openai,gemini,anthropic}()`: Extract normalised `FinishReason` (Stop/ToolUse/MaxTokens/Other)
- `resolve_tool_call()`: Shared — resolves dot-separated names and `activate_skill`

Key format differences:
| Aspect | OpenAI | Gemini | Anthropic |
|--------|--------|--------|-----------|
| Tool def key | `parameters` (wrapped in `function`) | `parameters` (in `functionDeclarations`) | `input_schema` (top-level) |
| Type casing | lowercase (`string`) | UPPERCASE (`STRING`) | lowercase (`string`) |
| Tool call location | `tool_calls[]` array on message | `functionCall` parts in content | `tool_use` content blocks |
| Tool result role | `tool` role message | `functionResponse` part in `user` | `tool_result` content block in `user` |
| System messages | `system` role | Separate `system_instruction` | Separate `system` param (skipped in messages) |
| Images | `image_url` with data URI | `inline_data` with `mime_type` | `image` source with `media_type` |
| PDFs | Text extraction fallback | `inline_data` | `document` source |
| Streaming | SSE `data:` lines, `[DONE]` terminal | SSE `data:` lines | Event-typed SSE (`event:` + `data:` pairs) |
| Token usage | `usage.prompt_tokens` / `completion_tokens` | `usageMetadata.promptTokenCount` / `candidatesTokenCount` | `usage.input_tokens` / `output_tokens` |
| Finish reason | `stop` / `tool_calls` / `length` | `STOP` / `MAX_TOKENS` | `end_turn` / `tool_use` / `max_tokens` |

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

**Purpose:** Proxy for Gemini API (avoids exposing API keys), backend for refreshing Google OAuth tokens (holds `GOOGLE_CLIENT_SECRET`), and session-based rate limiting for cloud model usage.

**Endpoints:**
- `POST /v1/llm/generate` — LLM completion (validates session token if provided, else falls back to per-call rate limiting)
- `POST /v1/usage/remaining` — Get remaining daily uses for all cloud models
- `POST /v1/usage/start-turn` — Create a generation session (checks rate limit, increments usage once, returns session token)
- `POST /v1/auth/google/refresh` — Refresh Google OAuth token

**LLM Completion Request:**
```typescript
POST /v1/llm/generate
Authorization: Bearer <supabase_access_token>

{
  modelType: "fast" | "pro",
  content: [{ role, parts }],  // Gemini format
  stream: boolean,
  systemPrompt?: string,
  jsonSchema?: object,
  tools?: any,
  sessionToken?: string,  // If present, validates session instead of per-call rate limiting
}
```

**Start Turn Request:**
```typescript
POST /v1/usage/start-turn
Authorization: Bearer <supabase_access_token>

{ "modelType": "fast" | "pro" }

// Returns: { session_token, max_calls, expires_at }
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
| Current user ID | OS keyring (`keyring` crate) | OS-level encryption | Bootstrap pointer — identifies active user on startup |
| Supabase refresh token | OS keyring (`keyring` crate) | OS-level encryption | Long-lived, high-value — most secure location |
| Google refresh token | OS keyring (`keyring` crate) | OS-level encryption | Long-lived, high-value — never touches disk |
| AES encryption key | OS keyring (`keyring` crate) | OS-level encryption | Protects session tokens in per-user store.json |
| DB encryption key (per-user) | OS keyring (`keyring` crate) | OS-level encryption | Per-user SQLCipher key as `db_key_{user_id}` |
| Supabase access token | `profiles/{user_id}/store.json` (AES-256-GCM) | Encrypted with keyring key | Short-lived — fast access without keyring I/O |
| Google access token | `profiles/{user_id}/store.json` (AES-256-GCM) | Encrypted with keyring key | Short-lived — fast access without keyring I/O |
| Session metadata (user, expiry) | `profiles/{user_id}/store.json` (plaintext) | None (non-sensitive) | Quick reads for UI state |
| User settings | `profiles/{user_id}/store.json` | None (non-sensitive) | Per-user preferences, isolated from other users |

Keyring service name: `"ambient"`. Entry names: `current_user_id`, `encryption_key`, `supabase_refresh_token`, `google_refresh_token`, `db_key_{user_id}` (per-user DB encryption keys).

**Key functions in `storage.rs`:**
- `store_session()`: Sets `current_user_id` in keyring, splits tokens — refresh tokens → keyring, access tokens → AES-encrypt → per-user store.json
- `retrieve_auth_state()`: Reads `current_user_id` from keyring → opens per-user store → decrypts tokens → reads refresh tokens from keyring → reconstructs `StoredAuthState`
- `get_current_user_id()`: Reads `current_user_id` from keyring (lightweight, no decryption)
- `get_refresh_token()` / `get_google_refresh_token()`: Read directly from keyring (no decryption needed)
- `get_access_token()` / `get_provider_token()`: Read from encrypted per-user store.json
- `clear_auth_state()`: Clears per-user store auth data + keyring entries (current_user_id, refresh tokens)

**Google Token Refresh Flow:**
1. `refresh_google_token()` acquires `REFRESH_MUTEX` to serialize refresh requests
2. If Supabase access token is expired, refreshes it first via `refresh_session_with_token()`
3. Reads Google refresh token from keyring (no decryption needed)
4. Sends refresh token to Cloudflare Worker (`/v1/auth/google/refresh`) which holds `GOOGLE_CLIENT_SECRET`
5. Worker calls Google's token endpoint and returns new access token
6. Stores updated session with new Google access token

## Security

### Sensitive Operations
- **Token storage:** Split architecture — refresh tokens in OS keyring (keychain), session tokens AES-256-GCM encrypted in per-user `profiles/{user_id}/store.json`. No global store.json — active user identified by `current_user_id` in keyring.
- **Database encryption:** Per-user SQLCipher databases with unique 32-byte keys in OS keyring. Raw hex key encoding (no PBKDF2 overhead). Each user's data is cryptographically isolated.
- **Per-user data isolation:** All user data (DB, settings, auth tokens, attachments) stored under `profiles/{user_id}/`. No global data files. Model weights and server are shared.
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
