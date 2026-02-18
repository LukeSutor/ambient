use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// HUD size options for user interface
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export, export_to = "settings.ts")]
pub enum HudSizeOption {
  Small,
  Normal,
  Large,
}

impl Default for HudSizeOption {
  fn default() -> Self {
    Self::Normal
  }
}

impl HudSizeOption {
  pub fn to_dimensions(&self) -> HudDimensions {
    match self {
      Self::Small => HudDimensions {
        chat_width: 400.0,
        input_bar_height: 106.0,
        chat_max_height: 250.0,
        login_width: 450.0,
        login_height: 600.0,
      },
      Self::Normal => HudDimensions {
        chat_width: 600.0,
        input_bar_height: 106.0,
        chat_max_height: 350.0,
        login_width: 450.0,
        login_height: 600.0,
      },
      Self::Large => HudDimensions {
        chat_width: 700.0,
        input_bar_height: 106.0,
        chat_max_height: 450.0,
        login_width: 450.0,
        login_height: 600.0,
      },
    }
  }

  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Small => "small",
      Self::Normal => "normal",
      Self::Large => "large",
    }
  }

  pub fn from_str(s: &str) -> Self {
    match s {
      "small" => Self::Small,
      "large" => Self::Large,
      _ => Self::Normal, // Default fallback
    }
  }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export, export_to = "settings.ts")]
pub struct HudDimensions {
  pub chat_width: f64,
  pub input_bar_height: f64,
  pub chat_max_height: f64,
  pub login_width: f64,
  pub login_height: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export, export_to = "settings.ts")]
pub enum HudState {
  Input,
  Chat,
  Login,
  Default,
}

// Model selection — stores the model's integer database id (as a string).
// Serialized as a plain string in settings (e.g. "1" for the default local model).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(transparent)]
#[ts(export, export_to = "settings.ts")]
pub struct ModelSelection(pub String);

impl Default for ModelSelection {
  fn default() -> Self {
    Self("1".to_string())
  }
}

impl ModelSelection {
  /// Returns the stored model id as a string.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "settings.ts")]
pub struct UserSettings {
  pub hud_size: HudSizeOption,
  pub show_full_thought_traces: bool,
  pub model_selection: ModelSelection,
  pub agent_config: crate::skills::types::AgentRuntimeConfig,
  /// List of skill names that the user has disabled.
  /// Skills in this list will not be available to the agentic runtime.
  #[serde(default)]
  pub disabled_skills: Vec<String>,
  /// Whether to offload model layers to GPU via Vulkan.
  /// Only effective when a compatible GPU is detected.
  #[serde(default)]
  pub gpu_acceleration: bool,
  /// How often the screen monitor polls for semantic automation triggers (seconds).
  /// Clamped to [5, 300]. Defaults to 30 when not set.
  #[serde(default)]
  pub screen_poll_interval_secs: Option<u64>,
}

impl Default for UserSettings {
  fn default() -> Self {
    Self {
      hud_size: HudSizeOption::default(),
      show_full_thought_traces: false,
      model_selection: ModelSelection::default(),
      agent_config: crate::skills::types::AgentRuntimeConfig::default(),
      disabled_skills: Vec::new(),
      gpu_acceleration: false,
      screen_poll_interval_secs: None,
    }
  }
}
