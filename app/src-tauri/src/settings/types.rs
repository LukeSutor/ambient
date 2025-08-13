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
                width: 400.0,
                collapsed_height: 50.0,
                expanded_height: 250.0,
            },
            Self::Normal => HudDimensions {
                width: 500.0,
                collapsed_height: 60.0,
                expanded_height: 350.0,
            },
            Self::Large => HudDimensions {
                width: 600.0,
                collapsed_height: 70.0,
                expanded_height: 450.0,
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
    pub width: f64,
    pub collapsed_height: f64,
    pub expanded_height: f64,
}


// Model selection
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export, export_to = "settings.ts")]
pub enum ModelSelection {
    Local,
    GptOss,
    Gpt5,
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
            Self::GptOss => "gpt_oss",
            Self::Gpt5 => "gpt_5",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "local" => Self::Local,
            "gpt_oss" => Self::GptOss,
            "gpt_5" => Self::Gpt5,
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
