//! Skill registry for managing available skills.
//!
//! The registry loads skill definitions from SKILL.md files in the
//! resources directory, parses them, and provides access for the
//! agentic runtime to query available skills and their tools.

use super::types::{Skill, SkillSummary, ToolDefinition, ToolParameter, ToolReturnType, ParameterType, AgentError};
use once_cell::sync::Lazy;
use serde_yaml::Value as YamlValue;
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;
use tauri::{AppHandle, Manager};

/// Global skill registry instance.
///
/// Uses RwLock for thread-safe access across the application.
/// Skills are loaded once at startup and cached.
static SKILL_REGISTRY: Lazy<RwLock<SkillRegistry>> = Lazy::new(|| {
    RwLock::new(SkillRegistry::new())
});

/// The skill registry manages all available skills.
///
/// It loads skills from the resources/skills directory,
/// parses their SKILL.md files, and provides methods
/// to query and retrieve skills.
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
}

impl SkillRegistry {
    /// Creates a new empty skill registry.
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Loads all bundled skills from the resources directory.
    ///
    /// Scans the resources/.skills directory for subdirectories
    /// containing SKILL.md files and loads them into the registry.
    pub fn load_bundled_skills(&mut self, app_handle: &AppHandle) -> Result<(), AgentError> {
        // Get the resource directory path
        let resource_path = app_handle
            .path()
            .resource_dir()
            .map_err(|e| AgentError::SkillParseError(format!("Failed to get resource dir: {}", e)))?
            .join(".skills");

        if !resource_path.exists() {
            log::warn!("[skills] No bundled skills directory found at {:?}", resource_path);
            return Ok(());
        }

        log::info!("[skills] Loading bundled skills from: {:?}", resource_path);

        // Read each subdirectory as a skill
        let entries = std::fs::read_dir(&resource_path)
            .map_err(|e| AgentError::SkillParseError(format!("Failed to read skills directory: {}", e)))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let skill_md_path = path.join("SKILL.md");
                if skill_md_path.exists() {
                    match self.load_skill_from_file(&skill_md_path) {
                        Ok(skill) => {
                            log::info!("[skills] Loaded skill: {}", skill.name);
                            self.skills.insert(skill.name.clone(), skill);
                        }
                        Err(e) => {
                            log::error!("[skills] Failed to load skill from {:?}: {}", path, e);
                        }
                    }
                } else {
                    log::debug!("[skills] No SKILL.md found in {:?}", path);
                }
            }
        }

        log::info!("[skills] Loaded {} skills total", self.skills.len());
        Ok(())
    }

    /// Parses a SKILL.md file into a Skill struct.
    ///
    /// SKILL.md files use YAML frontmatter for metadata
    /// followed by Markdown instructions.
    fn load_skill_from_file(&self, path: &Path) -> Result<Skill, AgentError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AgentError::SkillParseError(format!("Failed to read file: {}", e)))?;

        // Split frontmatter and body (delimited by ---)
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return Err(AgentError::SkillParseError(
                "Invalid SKILL.md format: missing frontmatter delimiters".to_string()
            ));
        }

        let frontmatter = parts[1].trim();
        let instructions = parts[2].trim().to_string();

        // Parse YAML frontmatter
        let yaml: YamlValue = serde_yaml::from_str(frontmatter)
            .map_err(|e| AgentError::SkillParseError(format!("Failed to parse YAML frontmatter: {}", e)))?;

        let name = yaml["name"]
            .as_str()
            .ok_or_else(|| AgentError::SkillParseError("Missing 'name' field".to_string()))?
            .to_string();

        let description = yaml["description"]
            .as_str()
            .ok_or_else(|| AgentError::SkillParseError("Missing 'description' field".to_string()))?
            .to_string();

        let version = yaml["version"]
            .as_str()
            .unwrap_or("1.0")
            .to_string();

        let requires_auth = yaml["requires_auth"]
            .as_bool()
            .unwrap_or(false);

        let requires_google_auth = yaml["requires_google_auth"]
            .as_bool()
            .unwrap_or(false);

        // Parse tools
        let tools = self.parse_tools(&yaml["tools"])?;

        Ok(Skill {
            name,
            description,
            version,
            requires_auth,
            requires_google_auth,
            tools,
            instructions,
        })
    }

    /// Parses tools from YAML value.
    fn parse_tools(&self, tools_yaml: &YamlValue) -> Result<Vec<ToolDefinition>, AgentError> {
        let mut tools = Vec::new();

        if let Some(tools_array) = tools_yaml.as_sequence() {
            for tool_yaml in tools_array {
                let name = tool_yaml["name"]
                    .as_str()
                    .ok_or_else(|| AgentError::SkillParseError("Tool missing 'name'".to_string()))?
                    .to_string();

                let description = tool_yaml["description"]
                    .as_str()
                    .ok_or_else(|| AgentError::SkillParseError("Tool missing 'description'".to_string()))?
                    .to_string();

                let parameters = self.parse_parameters(&tool_yaml["parameters"])?;

                let returns = if !tool_yaml["returns"].is_null() {
                    let rt = serde_json::from_value::<ToolReturnType>(
                        serde_yaml::from_value::<serde_json::Value>(
                            serde_yaml::to_value(tool_yaml["returns"].clone())
                                .map_err(|e| AgentError::SkillParseError(format!("Failed to convert returns: {}", e)))?
                        ).map_err(|e| AgentError::SkillParseError(format!("Failed to convert returns to JSON: {}", e)))?
                    ).map_err(|e| AgentError::SkillParseError(format!("Failed to parse returns: {}", e)))?;
                    Some(rt)
                } else {
                    None
                };

                tools.push(ToolDefinition {
                    skill_name: None, // Will be populated by the caller if needed
                    name,
                    description,
                    parameters,
                    returns,
                });
            }
        }

        Ok(tools)
    }

    /// Parses parameters from YAML value.
    fn parse_parameters(&self, params_yaml: &YamlValue) -> Result<Vec<ToolParameter>, AgentError> {
        let mut params = Vec::new();

        if let Some(params_map) = params_yaml.as_mapping() {
            for (key, value) in params_map {
                let name = key.as_str()
                    .ok_or_else(|| AgentError::SkillParseError("Parameter key must be string".to_string()))?
                    .to_string();

                let param_type_str = value["type"]
                    .as_str()
                    .unwrap_or("string");
                let param_type = match param_type_str {
                    "string" => ParameterType::String,
                    "integer" => ParameterType::Integer,
                    "number" => ParameterType::Number,
                    "boolean" => ParameterType::Boolean,
                    "array" => ParameterType::Array,
                    "object" => ParameterType::Object,
                    _ => ParameterType::String,
                };

                let description = value["description"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                let required = value["required"]
                    .as_bool()
                    .unwrap_or(false);

                let default = if !value["default"].is_null() {
                    let val = serde_yaml::from_value::<serde_json::Value>(
                        serde_yaml::to_value(value["default"].clone())
                            .map_err(|e| AgentError::SkillParseError(format!("Failed to convert default: {}", e)))?
                    ).map_err(|e| AgentError::SkillParseError(format!("Failed to convert default to JSON: {}", e)))?;
                    Some(val)
                } else {
                    None
                };

                params.push(ToolParameter {
                    name,
                    param_type,
                    description,
                    required,
                    default,
                });
            }
        }

        Ok(params)
    }

    /// Gets a skill by name.
    pub fn get_skill(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    /// Gets all skill summaries for progressive disclosure.
    pub fn get_all_summaries(&self) -> Vec<SkillSummary> {
        let mut summaries: Vec<SkillSummary> = self.skills.values().map(|s| s.to_summary()).collect();
        summaries.sort_by(|a, b| a.name.cmp(&b.name));
        summaries
    }

    /// Gets skill summaries filtered by authentication state and disabled skills.
    pub fn get_filtered_summaries(&self, is_google_authed: bool, disabled_skills: &[String]) -> Vec<SkillSummary> {
        let mut summaries: Vec<SkillSummary> = self.skills
            .values()
            .filter(|s| {
                if s.requires_google_auth && !is_google_authed {
                    return false;
                }
                if disabled_skills.contains(&s.name) {
                    return false;
                }
                true
            })
            .map(|s| s.to_summary())
            .collect();
        summaries.sort_by(|a, b| a.name.cmp(&b.name));
        summaries
    }

    /// Gets all available skill names.
    pub fn get_all_skill_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.skills.keys().cloned().collect();
        names.sort();
        names
    }

    /// Gets tools for a specific skill.
    pub fn get_skill_tools(&self, name: &str) -> Vec<ToolDefinition> {
        self.get_skill(name)
            .map(|s| s.tools.clone())
            .unwrap_or_default()
    }
}

// ============================================================================
// Public API Functions
// ============================================================================

/// Initializes the skill registry by loading bundled skills.
///
/// This should be called during app startup to populate the
/// registry with all available skills.
pub fn initialize_registry(app_handle: &AppHandle) -> Result<(), AgentError> {
    let mut registry = SKILL_REGISTRY
        .write()
        .map_err(|_| AgentError::RegistryNotInitialized)?;
    registry.load_bundled_skills(app_handle)
}

/// Gets a skill by name.
///
/// Returns a clone of the skill if found, None otherwise.
pub fn get_skill(name: &str) -> Option<Skill> {
    let registry = SKILL_REGISTRY.read().ok()?;
    registry.get_skill(name).cloned()
}

/// Gets all skill summaries.
///
/// Returns summary of all available skills in alphabetical order,
/// suitable for progressive disclosure to the LLM.
pub fn get_all_summaries() -> Vec<SkillSummary> {
    match SKILL_REGISTRY.read() {
        Ok(registry) => registry.get_all_summaries(),
        Err(_) => Vec::new(),
    }
}

/// Gets filtered skill summaries based on current user settings and auth state.
///
/// Loads disabled skills from settings and Google auth state internally,
/// so callers don't need to pass these values.
pub async fn get_filtered_summaries(app_handle: &AppHandle) -> Vec<SkillSummary> {
    // Load current settings to get disabled skills
    let disabled_skills = match crate::settings::service::load_user_settings(app_handle.clone()).await {
        Ok(settings) => settings.disabled_skills,
        Err(e) => {
            log::warn!("[skills] Failed to load settings for filtering: {}", e);
            Vec::new()
        }
    };

    // Check Google auth state
    let is_google_authed = match crate::auth::commands::get_auth_state(app_handle.clone()).await {
        Ok(state) => state.is_google_authenticated,
        Err(_) => false,
    };

    match SKILL_REGISTRY.read() {
        Ok(registry) => registry.get_filtered_summaries(is_google_authed, &disabled_skills),
        Err(_) => Vec::new(),
    }
}

/// Gets tools for a specific skill.
///
/// Returns all tools defined for the given skill.
pub fn get_skill_tools(name: &str) -> Vec<ToolDefinition> {
    match SKILL_REGISTRY.read() {
        Ok(registry) => registry.get_skill_tools(name),
        Err(_) => Vec::new(),
    }
}

/// Gets all available skill names.
///
/// Returns list of all skill identifiers in alphabetical order.
pub fn get_all_skill_names() -> Vec<String> {
    match SKILL_REGISTRY.read() {
        Ok(registry) => registry.get_all_skill_names(),
        Err(_) => Vec::new(),
    }
}

/// Tauri command to get available skills.
///
/// Returns all skill summaries in alphabetical order for the frontend to display.
#[tauri::command]
pub fn get_available_skills() -> Vec<SkillSummary> {
    get_all_summaries()
}

/// Tauri command to get tools for a specific skill.
///
/// Returns all tools defined for the given skill.
#[tauri::command]
pub fn get_skill_tools_command(name: String) -> Vec<ToolDefinition> {
    get_skill_tools(&name)
}

/// Checks if a skill exists.
///
/// Returns true if the skill is registered.
pub fn skill_exists(name: &str) -> bool {
    match SKILL_REGISTRY.read() {
        Ok(registry) => registry.get_skill(name).is_some(),
        Err(_) => false,
    }
}

/// Checks if a skill requires Google authentication.
///
/// Returns true if the skill exists and requires Google auth.
pub fn skill_requires_google_auth(name: &str) -> bool {
    match SKILL_REGISTRY.read() {
        Ok(registry) => registry.get_skill(name)
            .map(|s| s.requires_google_auth)
            .unwrap_or(false),
        Err(_) => false,
    }
}
