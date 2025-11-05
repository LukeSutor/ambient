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
                default_width: 200.0,
                default_height: 200.0,
                chat_width: 400.0,
                input_bar_height: 50.0,
                chat_max_height: 250.0,
                login_width: 300.0,
                login_height: 200.0,
            },
            Self::Normal => HudDimensions {
                default_width: 200.0,
                default_height: 200.0,
                chat_width: 500.0,
                input_bar_height: 60.0,
                chat_max_height: 350.0,
                login_width: 400.0,
                login_height: 300.0,
            },
            Self::Large => HudDimensions {
                default_width: 200.0,
                default_height: 200.0,
                chat_width: 600.0,
                input_bar_height: 70.0,
                chat_max_height: 450.0,
                login_width: 500.0,
                login_height: 400.0,
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
    pub default_width: f64,
    pub default_height: f64,
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
    Default
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
