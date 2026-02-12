//! Types for the browser-use runtime.

/// Configuration for the browser-use runtime.
#[derive(Debug, Clone)]
pub struct BrowserUseConfig {
    /// Maximum number of iterations before giving up.
    pub max_iterations: usize,
    /// Number of messages to include in context for local model.
    pub local_context_limit: usize,
    /// Number of messages to include in context for cloud model.
    pub cloud_context_limit: usize,
    /// Delay in milliseconds after each action before taking a snapshot.
    pub action_delay_ms: u64,
    /// Delay in milliseconds after navigation before taking a snapshot.
    pub navigation_delay_ms: u64,
    /// Starting URL for the browser session.
    pub start_url: String,
    /// Timeout for snapshot extraction in seconds.
    pub snapshot_timeout_secs: u64,
    /// Maximum tool calls per turn.
    pub max_tool_calls_per_turn: usize,
}

impl Default for BrowserUseConfig {
    fn default() -> Self {
        Self {
            max_iterations: 30,
            local_context_limit: 10,
            cloud_context_limit: 20,
            action_delay_ms: 1500,
            navigation_delay_ms: 2500,
            start_url: "https://www.google.com".to_string(),
            snapshot_timeout_secs: 10,
            max_tool_calls_per_turn: 3,
        }
    }
}

impl BrowserUseConfig {
    /// Returns the context limit for the given provider type.
    pub fn context_limit_for(&self, is_local: bool) -> usize {
        if is_local {
            self.local_context_limit
        } else {
            self.cloud_context_limit
        }
    }
}
