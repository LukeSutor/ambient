//! Shared constants used across the application.

// Directory names
pub const EMBEDDING_DIR: &str = "models/embedding";
pub const VLM_DIR: &str = "models/vlm";
pub const LLM_DIR: &str = "models/llm";

// VLM model filenames
pub const TEXT_FILE: &str = "text-model.gguf";
pub const MMPROJ_FILE: &str = "mmproj-model.gguf";

// LLM model filename
pub const LLM_FILE: &str = "SmolLM3-3B-Q6_K.gguf";

// LLM model download link (example - update with actual model)
pub const LLM_LINK: &str = "https://huggingface.co/unsloth/SmolLM3-3B-GGUF/resolve/main/SmolLM3-3B-Q6_K.gguf";

// VLM model download links
// pub const TEXT_LINK: &str = "https://huggingface.co/ggml-org/SmolVLM2-500M-Video-Instruct-GGUF/resolve/main/SmolVLM2-500M-Video-Instruct-Q8_0.gguf";
// pub const MMPROJ_LINK: &str = "https://huggingface.co/ggml-org/SmolVLM2-500M-Video-Instruct-GGUF/resolve/main/mmproj-SmolVLM2-500M-Video-Instruct-f16.gguf";
pub const TEXT_LINK: &str = "https://huggingface.co/ggml-org/SmolVLM2-2.2B-Instruct-GGUF/resolve/main/SmolVLM2-2.2B-Instruct-Q4_K_M.gguf";
pub const MMPROJ_LINK: &str = "https://huggingface.co/ggml-org/SmolVLM2-2.2B-Instruct-GGUF/resolve/main/mmproj-SmolVLM2-2.2B-Instruct-Q8_0.gguf";

// Chat template for VLM
pub const VLM_CHAT_TEMPLATE: &str = "smolvlm";

// Embedding model name (used internally by fastembed, but good to have for reference)
// pub const EMBEDDING_MODEL_NAME: &str = "AllMiniLML6V2"; // Example if needed later
