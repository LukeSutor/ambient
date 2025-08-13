//! Shared constants used across the application.
use tokio::time::Duration;

// Directory names
pub const VLM_DIR: &str = "models/vlm";
pub const LLM_DIR: &str = "models/llm";

// VLM model filenames
pub const TEXT_FILE: &str = "text-model.gguf";
pub const MMPROJ_FILE: &str = "mmproj-model.gguf";

// LLM model filename
pub const LLM_FILE: &str = "Qwen3-1.7B-Q4_K_M.gguf";

// LLM model download link (example - update with actual model)
pub const LLM_LINK: &str =
  "https://huggingface.co/unsloth/SmolLM3-3B-GGUF/resolve/main/SmolLM3-3B-Q6_K.gguf";

// VLM model download links
pub const TEXT_LINK: &str = "https://huggingface.co/ggml-org/SmolVLM2-2.2B-Instruct-GGUF/resolve/main/SmolVLM2-2.2B-Instruct-Q4_K_M.gguf";
pub const MMPROJ_LINK: &str = "https://huggingface.co/ggml-org/SmolVLM2-2.2B-Instruct-GGUF/resolve/main/mmproj-SmolVLM2-2.2B-Instruct-Q8_0.gguf";

// Chat template for VLM
pub const VLM_CHAT_TEMPLATE: &str = "smolvlm";

// Settings storage
pub const SETTINGS_STORE_PATH: &str = "user-settings.json";
pub const SETTINGS_KEY: &str = "settings";

// Server setup
pub const MAX_PORT: u16 = 9999;
pub const MIN_PORT: u16 = 8000;
pub const MAX_PORT_ATTEMPTS: u8 = 20;
pub const HEALTH_CHECK_ENDPOINT: &str = "/health";
pub const MAX_HEALTH_CHECK_RETRIES: u8 = 30;
pub const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(120);