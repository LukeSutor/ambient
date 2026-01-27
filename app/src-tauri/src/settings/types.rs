use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// HUD size options for the user interface
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
        input_bar_height: 130.0,
        chat_max_height: 250.0,
        login_width: 450.0,
        login_height: 600.0,
      },
      Self::Normal => HudDimensions {
        chat_width: 600.0,
        input_bar_height: 130.0,
        chat_max_height: 350.0,
        login_width: 450.0,
        login_height: 600.0,
      },
      Self::Large => HudDimensions {
        chat_width: 700.0,
        input_bar_height: 130.0,
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

// Model selection
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export, export_to = "settings.ts")]
pub enum ModelSelection {
  Local,
  Fast,
  Pro,
}

impl Default for ModelSelection {
  fn default() -> Self {
    Self::Local
  }
}

impl ModelSelection {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Local => "local",
      Self::Fast => "fast",
      Self::Pro => "pro",
    }
  }

  pub fn from_str(s: &str) -> Self {
    match s {
      "local" => Self::Local,
      "fast" => Self::Fast,
      "pro" => Self::Pro,
      _ => Self::Local, // Default fallback
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "settings.ts")]
pub struct UserSettings {
  pub hud_size: HudSizeOption,
  pub model_selection: ModelSelection,
}

impl Default for UserSettings {
  fn default() -> Self {
    Self {
      hud_size: HudSizeOption::default(),
      model_selection: ModelSelection::default(),
    }
  }
}
