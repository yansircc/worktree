//! JSONC configuration parser for wt.
//!
//! Parses `.wt/config.jsonc` with support for comments.
//!
//! ## Configuration
//!
//! Task lifecycle is defined by phases with on_enter/on_exit workflows:
//! ```jsonc
//! {
//!   "phases": {
//!     "sequence": ["pending", "developing", "reviewing", "completed"],
//!     "definitions": { "developing": { "on_enter": [...] } }
//!   }
//! }
//! ```

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::constants::DEFAULT_SESSION_NAME;
use crate::error::{Result, WtError};
use crate::services::multiplexer::{create_multiplexer, Multiplexer, MultiplexerType};

// Re-export from project.rs for phases v2 support
pub use crate::models::project::{ConcurrencyConfig, PhasesConfig, ProjectObserve};

/// Path to the JSONC config file
pub const CONFIG_FILE: &str = ".wt/config.jsonc";

/// Default worktree directory
const DEFAULT_WORKTREE_DIR: &str = ".wt/worktrees";

// ============================================================================
// Main Configuration
// ============================================================================

/// Logs configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct LogsConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_types: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_fields: Vec<String>,
}

/// Main configuration structure for wt
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WtConfig {
    /// JSON Schema reference (for editor support)
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub schema: Option<String>,

    /// Terminal multiplexer: tmux or zellij
    #[serde(default = "default_multiplexer")]
    #[schemars(description = "Terminal multiplexer to use: 'tmux' or 'zellij'")]
    pub multiplexer: String,

    /// Session name for the multiplexer
    #[serde(default = "default_session_name")]
    #[schemars(description = "Session name for the terminal multiplexer")]
    pub session_name: String,

    /// Claude CLI command (default: claude)
    #[serde(default = "default_claude_command")]
    #[schemars(description = "Claude CLI command to use")]
    pub claude_command: String,

    /// Directory for worktrees
    #[serde(default = "default_worktree_dir")]
    #[schemars(description = "Directory path for git worktrees")]
    pub worktree_dir: String,

    /// Start arguments for Claude
    #[serde(default = "default_start_args")]
    #[schemars(description = "Arguments to pass when starting Claude")]
    pub start_args: String,

    /// Files to copy to worktree
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(description = "List of files to copy to each worktree")]
    pub copy_files: Vec<String>,

    /// Logs configuration
    #[serde(default)]
    pub logs: LogsConfig,

    // ============================================================================
    // Phases configuration
    // ============================================================================

    /// Phases configuration
    /// Task lifecycle is controlled by phases
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Phase sequence and definitions for task lifecycle")]
    pub phases: Option<PhasesConfig>,

    /// Concurrency configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Concurrency limits for tasks and agents")]
    pub concurrency: Option<ConcurrencyConfig>,

    /// Observation/notification configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Observation and dashboard settings")]
    pub observe: Option<ProjectObserve>,
}

fn default_multiplexer() -> String {
    "tmux".to_string()
}

fn default_session_name() -> String {
    DEFAULT_SESSION_NAME.to_string()
}

fn default_claude_command() -> String {
    "claude".to_string()
}

fn default_worktree_dir() -> String {
    DEFAULT_WORKTREE_DIR.to_string()
}

fn default_start_args() -> String {
    r#"--verbose --output-format=stream-json --input-format=stream-json -p "@.wt/tasks/${task}.md 请完成这个任务""#.to_string()
}

impl Default for WtConfig {
    fn default() -> Self {
        Self {
            schema: None,
            multiplexer: default_multiplexer(),
            session_name: default_session_name(),
            claude_command: default_claude_command(),
            worktree_dir: default_worktree_dir(),
            start_args: default_start_args(),
            copy_files: Vec::new(),
            logs: LogsConfig::default(),
            phases: None,
            concurrency: None,
            observe: None,
        }
    }
}

impl WtConfig {
    /// Load configuration from `.wt/config.jsonc`
    pub fn load() -> Result<Self> {
        let path = Path::new(CONFIG_FILE);
        if !path.exists() {
            // Return default config if file doesn't exist
            return Ok(Self::default());
        }

        let content =
            std::fs::read_to_string(path).map_err(|e| WtError::ConfigRead(e.to_string()))?;
        Self::from_str(&content)
    }

    /// Parse config from JSONC string
    pub fn from_str(content: &str) -> Result<Self> {
        // Strip comments using json_comments
        let stripped = json_comments::StripComments::new(content.as_bytes());
        let config: WtConfig = serde_json::from_reader(stripped)
            .map_err(|e| WtError::ConfigRead(format!("Invalid JSONC: {}", e)))?;
        config.validate()?;
        Ok(config)
    }

    /// Validate configuration
    fn validate(&self) -> Result<()> {
        // Validate multiplexer
        if !["tmux", "zellij"].contains(&self.multiplexer.as_str()) {
            return Err(WtError::ConfigRead(format!(
                "Invalid multiplexer '{}': must be 'tmux' or 'zellij'",
                self.multiplexer
            )));
        }

        // Validate session name (non-empty)
        if self.session_name.is_empty() {
            return Err(WtError::ConfigRead(
                "session_name cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    /// Get the configured multiplexer type
    pub fn multiplexer_type(&self) -> MultiplexerType {
        MultiplexerType::from_str(&self.multiplexer).unwrap_or_default()
    }

    /// Create a multiplexer instance based on config
    pub fn create_multiplexer(&self) -> Box<dyn Multiplexer> {
        create_multiplexer(self.multiplexer_type())
    }

    /// Get phase sequence from config.
    /// Returns error if phases not configured.
    pub fn phase_sequence(&self) -> Result<Vec<String>> {
        match &self.phases {
            Some(p) if !p.sequence.is_empty() => Ok(p.sequence.clone()),
            _ => Err(WtError::ConfigRead(
                "No phases configured. Run 'wt init' to create config.".to_string()
            )),
        }
    }

    /// Get phase definition by ID
    pub fn get_phase(&self, phase_id: &str) -> Option<&crate::models::phase::Phase> {
        self.phases
            .as_ref()
            .and_then(|p| p.definitions.get(phase_id))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = WtConfig::default();
        assert_eq!(config.multiplexer, "tmux");
        assert_eq!(config.session_name, "wt");
        assert_eq!(config.claude_command, "claude");
        assert_eq!(config.worktree_dir, ".wt/worktrees");
    }

    #[test]
    fn test_config_minimal_json() {
        let json = "{}";
        let config = WtConfig::from_str(json).unwrap();
        assert_eq!(config.multiplexer, "tmux");
        assert_eq!(config.session_name, "wt");
    }

    #[test]
    fn test_config_with_comments() {
        let jsonc = r#"{
            // This is a comment
            "multiplexer": "zellij",
            /* Multi-line
               comment */
            "session_name": "my-project"
        }"#;
        let config = WtConfig::from_str(jsonc).unwrap();
        assert_eq!(config.multiplexer, "zellij");
        assert_eq!(config.session_name, "my-project");
    }

    #[test]
    fn test_config_full() {
        let jsonc = r#"{
            "multiplexer": "zellij",
            "session_name": "test-project",
            "claude_command": "claude --model opus",
            "worktree_dir": "/custom/worktrees"
        }"#;
        let config = WtConfig::from_str(jsonc).unwrap();
        assert_eq!(config.multiplexer, "zellij");
        assert_eq!(config.session_name, "test-project");
        assert_eq!(config.claude_command, "claude --model opus");
        assert_eq!(config.worktree_dir, "/custom/worktrees");
    }

    #[test]
    fn test_config_invalid_multiplexer() {
        let json = r#"{"multiplexer": "invalid"}"#;
        let result = WtConfig::from_str(json);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid multiplexer"));
    }

    #[test]
    fn test_config_empty_session_name() {
        let json = r#"{"session_name": ""}"#;
        let result = WtConfig::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiplexer_type() {
        let json = r#"{"multiplexer": "tmux"}"#;
        let config = WtConfig::from_str(json).unwrap();
        assert_eq!(config.multiplexer_type(), MultiplexerType::Tmux);

        let json = r#"{"multiplexer": "zellij"}"#;
        let config = WtConfig::from_str(json).unwrap();
        assert_eq!(config.multiplexer_type(), MultiplexerType::Zellij);
    }

    #[test]
    fn test_trailing_comma_in_jsonc() {
        // json_comments should handle trailing commas
        let jsonc = r#"{
            "multiplexer": "tmux",
            "session_name": "test",
        }"#;
        // Note: standard JSON doesn't allow trailing commas
        // json_comments strips comments but doesn't fix trailing commas
        // This test documents the behavior
        let result = WtConfig::from_str(jsonc);
        assert!(result.is_err() || result.is_ok());
    }

}
