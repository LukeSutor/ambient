//! Shared constants used across the application.
use tokio::time::Duration;

// Directory names
pub const VLM_DIR: &str = "models/vlm";
pub const LLM_DIR: &str = "models/llm";
pub const OCR_DIR: &str = "models/ocr";
pub const EMBEDDING_DIR: &str = "models/embedding";

// VLM model filenames
pub const TEXT_FILE: &str = "Qwen3VL-2B-Instruct-Q4_K_M.gguf";
pub const MMPROJ_FILE: &str = "mmproj-Qwen3VL-2B-Instruct-Q8_0.gguf";

// LLM model filename
pub const LLM_FILE: &str = "Qwen3-1.7B-Q4_K_M.gguf";

// OCR model filenames
pub const TEXT_DETECTION_FILE: &str = "text-detection.rten";
pub const TEXT_RECOGNITION_FILE: &str = "text-recognition.rten";

// Embedding model filename
pub const EMBEDDING_FILE: &str = "embeddinggemma-300m-Q4_0.gguf";

// VLM model download links
pub const TEXT_LINK: &str = "https://huggingface.co/Qwen/Qwen3-VL-2B-Instruct-GGUF/resolve/main/Qwen3VL-2B-Instruct-Q4_K_M.gguf";
pub const MMPROJ_LINK: &str = "https://huggingface.co/Qwen/Qwen3-VL-2B-Instruct-GGUF/resolve/main/mmproj-Qwen3VL-2B-Instruct-Q8_0.gguf";

// Chat template for VLM
pub const VLM_CHAT_TEMPLATE: &str = "smolvlm";

// Settings storage
pub const STORE_PATH: &str = "store.json";
pub const SETTINGS_KEY: &str = "settings";
pub const AUTH_KEY: &str = "auth";

// Keyring constants
pub const KEYRING_ENCRYPTION_KEY: &str = "supabase_storage_key";
pub const KEYRING_SERVICE: &str = "local-computer-use";
pub const KEYRING_AUTH_KEY: &str = "supabase_auth";

// Server setup
pub const MAX_PORT: u16 = 9999;
pub const MIN_PORT: u16 = 8000;
pub const MAX_PORT_ATTEMPTS: u8 = 20;
pub const HEALTH_CHECK_ENDPOINT: &str = "/health";
pub const MAX_HEALTH_CHECK_RETRIES: u8 = 30;
pub const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(120);

// HUD information
pub const HUD_WINDOW_LABEL: &str = "main";

// Dashboard information
pub const DASHBOARD_WINDOW_LABEL: &str = "secondary";
pub const DASHBOARD_PATH: &str = "/secondary";

// Computer use toast window information
pub const COMPUTER_USE_WINDOW_LABEL: &str = "computer-use";
pub const COMPUTER_USE_PATH: &str = "/computer-use";
pub const MARGIN_LEFT: u32 = 50;
pub const MARGIN_BOTTOM: u32 = 20;

// Cost, water, and energy estimates per token
pub const COST_PER_TOKEN: f64 = 0.000004375; // USD/token
pub const WATER_PER_TOKEN: f64 = 0.0033; // mL/token
pub const ENERGY_PER_TOKEN: f64 = 0.0024; // Wh/token
