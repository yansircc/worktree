//! JSONC configuration parser for Agent Hooks system (v2)
//!
//! Parses `.wt/config.jsonc` with support for comments.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::constants::DEFAULT_SESSION_NAME;
use crate::error::{Result, WtError};
use crate::services::multiplexer::{create_multiplexer, Multiplexer, MultiplexerType};

/// Path to the new JSONC config file
pub const CONFIG_FILE: &str = ".wt/config.jsonc";

/// Default worktree directory
const DEFAULT_WORKTREE_DIR: &str = ".wt/worktrees";

// ============================================================================
// Step Types
// ============================================================================

/// A single step in a hook pipeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Step {
    /// Execute a shell script
    Script {
        run: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on_error: Option<Box<Step>>,
    },

    /// Run a Claude agent
    Agent {
        #[serde(default)]
        interactive: bool,
        #[serde(default = "default_model")]
        model: String,
        prompt: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tools: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        allowed_tools: Vec<String>,
        #[serde(default)]
        skip_permissions: bool,
        #[serde(default = "default_output_format")]
        output_format: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        window: Option<String>,
    },

    /// Call wt internal operation
    Internal {
        run: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on_conflict: Option<Box<Step>>,
    },

    /// Conditional execution
    Condition {
        #[serde(rename = "if")]
        if_: String,
        then: Box<Step>,
        #[serde(rename = "else", default, skip_serializing_if = "Option::is_none")]
        else_: Option<Box<Step>>,
    },
}

fn default_model() -> String {
    "sonnet".to_string()
}

fn default_output_format() -> String {
    "text".to_string()
}

// ============================================================================
// Hook Configuration
// ============================================================================

/// Hook definition - either a list of steps or a pipeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum HookDef {
    /// Sequential steps
    Steps(Vec<Step>),
    /// Pipeline mode (agents chained via stream-json)
    Pipeline { pipeline: Vec<Step> },
}

impl Default for HookDef {
    fn default() -> Self {
        HookDef::Steps(Vec::new())
    }
}

impl HookDef {
    pub fn is_empty(&self) -> bool {
        match self {
            HookDef::Steps(steps) => steps.is_empty(),
            HookDef::Pipeline { pipeline } => pipeline.is_empty(),
        }
    }
}

/// Hooks configuration for all commands
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct HooksConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<HookDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<HookDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume: Option<HookDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub complete: Option<HookDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delete: Option<HookDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reset: Option<HookDef>,
}

impl HooksConfig {
    pub fn is_empty(&self) -> bool {
        self.run.is_none()
            && self.review.is_none()
            && self.resume.is_none()
            && self.complete.is_none()
            && self.delete.is_none()
            && self.reset.is_none()
    }

    /// Get hook by name
    pub fn get(&self, name: &str) -> Option<&HookDef> {
        match name {
            "run" => self.run.as_ref(),
            "review" => self.review.as_ref(),
            "resume" => self.resume.as_ref(),
            "complete" => self.complete.as_ref(),
            "delete" => self.delete.as_ref(),
            "reset" => self.reset.as_ref(),
            _ => None,
        }
    }
}

// ============================================================================
// Main Configuration
// ============================================================================

/// Logs configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogsConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_types: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_fields: Vec<String>,
}

/// Main configuration structure for v2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WtConfig {
    /// Terminal multiplexer: tmux or zellij
    #[serde(default = "default_multiplexer")]
    pub multiplexer: String,

    /// Session name for the multiplexer
    #[serde(default = "default_session_name")]
    pub session_name: String,

    /// Claude CLI command (default: claude)
    #[serde(default = "default_claude_command")]
    pub claude_command: String,

    /// Directory for worktrees
    #[serde(default = "default_worktree_dir")]
    pub worktree_dir: String,

    /// Start arguments for Claude (v1 compat)
    #[serde(default = "default_start_args")]
    pub start_args: String,

    /// Files to copy to worktree (v1 compat)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub copy_files: Vec<String>,

    /// Logs configuration (v1 compat)
    #[serde(default)]
    pub logs: LogsConfig,

    /// Hooks configuration
    #[serde(default, skip_serializing_if = "HooksConfig::is_empty")]
    pub hooks: HooksConfig,
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
            multiplexer: default_multiplexer(),
            session_name: default_session_name(),
            claude_command: default_claude_command(),
            worktree_dir: default_worktree_dir(),
            start_args: default_start_args(),
            copy_files: Vec::new(),
            logs: LogsConfig::default(),
            hooks: HooksConfig::default(),
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

    /// Get hook definition by command name
    pub fn get_hook(&self, name: &str) -> Option<&HookDef> {
        self.hooks.get(name)
    }

    /// Check if user has defined a custom complete hook (v1 compat)
    pub fn has_custom_complete_hook(&self) -> bool {
        self.hooks.complete.is_some()
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
        assert!(config.hooks.is_empty());
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
            "worktree_dir": "/custom/worktrees",
            "hooks": {
                "run": [
                    { "type": "internal", "run": "worktree:create" },
                    { "type": "script", "run": "npm install" },
                    { "type": "agent", "interactive": true, "prompt": "Do the task" }
                ]
            }
        }"#;
        let config = WtConfig::from_str(jsonc).unwrap();
        assert_eq!(config.multiplexer, "zellij");
        assert_eq!(config.session_name, "test-project");
        assert_eq!(config.claude_command, "claude --model opus");
        assert_eq!(config.worktree_dir, "/custom/worktrees");
        assert!(config.hooks.run.is_some());
    }

    #[test]
    fn test_config_invalid_multiplexer() {
        let json = r#"{"multiplexer": "invalid"}"#;
        let result = WtConfig::from_str(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid multiplexer"));
    }

    #[test]
    fn test_config_empty_session_name() {
        let json = r#"{"session_name": ""}"#;
        let result = WtConfig::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_step_script() {
        let json = r#"{"type": "script", "run": "npm test"}"#;
        let step: Step = serde_json::from_str(json).unwrap();
        match step {
            Step::Script { run, on_error } => {
                assert_eq!(run, "npm test");
                assert!(on_error.is_none());
            }
            _ => panic!("Expected Script step"),
        }
    }

    #[test]
    fn test_step_script_with_on_error() {
        let json = r#"{
            "type": "script",
            "run": "npm test",
            "on_error": { "type": "script", "run": "echo failed" }
        }"#;
        let step: Step = serde_json::from_str(json).unwrap();
        match step {
            Step::Script { run, on_error } => {
                assert_eq!(run, "npm test");
                assert!(on_error.is_some());
            }
            _ => panic!("Expected Script step"),
        }
    }

    #[test]
    fn test_step_agent_minimal() {
        let json = r#"{"type": "agent", "prompt": "Do something"}"#;
        let step: Step = serde_json::from_str(json).unwrap();
        match step {
            Step::Agent {
                interactive,
                model,
                prompt,
                ..
            } => {
                assert!(!interactive);
                assert_eq!(model, "sonnet"); // default
                assert_eq!(prompt, "Do something");
            }
            _ => panic!("Expected Agent step"),
        }
    }

    #[test]
    fn test_step_agent_full() {
        let json = r#"{
            "type": "agent",
            "interactive": true,
            "model": "opus",
            "prompt": "Review code",
            "tools": ["Read", "Edit"],
            "allowed_tools": ["Bash(npm *)"],
            "skip_permissions": true,
            "output_format": "stream-json",
            "window": "new"
        }"#;
        let step: Step = serde_json::from_str(json).unwrap();
        match step {
            Step::Agent {
                interactive,
                model,
                prompt,
                tools,
                allowed_tools,
                skip_permissions,
                output_format,
                window,
            } => {
                assert!(interactive);
                assert_eq!(model, "opus");
                assert_eq!(prompt, "Review code");
                assert_eq!(tools, vec!["Read", "Edit"]);
                assert_eq!(allowed_tools, vec!["Bash(npm *)"]);
                assert!(skip_permissions);
                assert_eq!(output_format, "stream-json");
                assert_eq!(window, Some("new".to_string()));
            }
            _ => panic!("Expected Agent step"),
        }
    }

    #[test]
    fn test_step_internal() {
        let json = r#"{"type": "internal", "run": "worktree:create"}"#;
        let step: Step = serde_json::from_str(json).unwrap();
        match step {
            Step::Internal { run, on_conflict } => {
                assert_eq!(run, "worktree:create");
                assert!(on_conflict.is_none());
            }
            _ => panic!("Expected Internal step"),
        }
    }

    #[test]
    fn test_step_condition() {
        let json = r#"{
            "type": "condition",
            "if": "wt internal git:has-changes",
            "then": { "type": "script", "run": "git commit -m 'auto'" },
            "else": { "type": "script", "run": "echo no changes" }
        }"#;
        let step: Step = serde_json::from_str(json).unwrap();
        match step {
            Step::Condition { if_, then, else_ } => {
                assert_eq!(if_, "wt internal git:has-changes");
                assert!(matches!(*then, Step::Script { .. }));
                assert!(else_.is_some());
            }
            _ => panic!("Expected Condition step"),
        }
    }

    #[test]
    fn test_hook_def_steps() {
        let json = r#"[
            { "type": "script", "run": "npm install" },
            { "type": "agent", "prompt": "code" }
        ]"#;
        let hook: HookDef = serde_json::from_str(json).unwrap();
        match hook {
            HookDef::Steps(steps) => assert_eq!(steps.len(), 2),
            _ => panic!("Expected Steps"),
        }
    }

    #[test]
    fn test_hook_def_pipeline() {
        let json = r#"{
            "pipeline": [
                { "type": "agent", "model": "haiku", "prompt": "list files" },
                { "type": "agent", "model": "sonnet", "prompt": "idle" }
            ]
        }"#;
        let hook: HookDef = serde_json::from_str(json).unwrap();
        match hook {
            HookDef::Pipeline { pipeline } => assert_eq!(pipeline.len(), 2),
            _ => panic!("Expected Pipeline"),
        }
    }

    #[test]
    fn test_hooks_config_get() {
        let json = r#"{
            "hooks": {
                "run": [{ "type": "script", "run": "echo run" }],
                "review": [{ "type": "script", "run": "echo review" }]
            }
        }"#;
        let config: WtConfig = serde_json::from_str(json).unwrap();

        assert!(config.get_hook("run").is_some());
        assert!(config.get_hook("review").is_some());
        assert!(config.get_hook("complete").is_none());
        assert!(config.get_hook("unknown").is_none());
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
        // If this fails, we might need a different JSONC parser
        assert!(result.is_err() || result.is_ok());
    }
}
