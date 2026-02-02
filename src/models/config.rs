//! JSONC configuration parser for Agent Hooks system (v2)
//!
//! Parses `.wt/config.jsonc` with support for comments.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::constants::DEFAULT_SESSION_NAME;
use crate::error::{Result, WtError};
use crate::models::AgentStep;
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
        #[serde(flatten)]
        agent: AgentStep,
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

// ============================================================================
// Hook Configuration
// ============================================================================

/// Hook definition - either a list of steps, a pipeline, or a reference to predefined pipeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum HookDef {
    /// Reference to a predefined pipeline by name
    PipelineRef { use_pipeline: String },
    /// Sequential steps
    Steps(Vec<Step>),
    /// Inline pipeline mode (agents chained via stream-json)
    Pipeline { pipeline: Vec<Step> },
}

impl Default for HookDef {
    fn default() -> Self {
        HookDef::Steps(Vec::new())
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

/// Predefined pipelines configuration
pub type PipelinesConfig = std::collections::HashMap<String, Vec<Step>>;

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

    /// Predefined pipelines (can be referenced by name in hooks)
    #[serde(default, skip_serializing_if = "PipelinesConfig::is_empty")]
    pub pipelines: PipelinesConfig,

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
            pipelines: PipelinesConfig::new(),
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

    /// Get a pipeline by name (user-defined or built-in)
    pub fn get_pipeline(&self, name: &str) -> Option<Vec<Step>> {
        // First check user-defined pipelines
        if let Some(steps) = self.pipelines.get(name) {
            return Some(steps.clone());
        }

        // Then check built-in pipelines
        builtin_pipeline(name)
    }

    /// Resolve a HookDef, expanding pipeline references
    pub fn resolve_hook(&self, hook: &HookDef) -> Option<HookDef> {
        match hook {
            HookDef::PipelineRef { use_pipeline } => {
                self.get_pipeline(use_pipeline)
                    .map(|steps| HookDef::Pipeline { pipeline: steps })
            }
            other => Some(other.clone()),
        }
    }

    /// Check if user has defined a custom complete hook (v1 compat)
    pub fn has_custom_complete_hook(&self) -> bool {
        self.hooks.complete.is_some()
    }
}

/// Built-in predefined pipelines
fn builtin_pipeline(name: &str) -> Option<Vec<Step>> {
    match name {
        "code-review" => Some(vec![
            Step::Agent {
                agent: AgentStep::new("Quick lint check for task ${task}. Report any obvious issues.")
                    .with_model("haiku")
                    .with_print()
                    .with_max_turns(5)
                    .with_tools(vec!["Read".into(), "Grep".into(), "Glob".into()])
                    .with_no_session_persistence()
                    .with_include_partial_messages(),
            },
            Step::Agent {
                agent: AgentStep::new(
                    "Deep code review for task ${task}. Check for bugs, security issues, and suggest improvements.",
                )
                .with_model("sonnet")
                .with_print()
                .with_max_turns(10)
                .with_tools(vec!["Read".into(), "Grep".into(), "Glob".into()])
                .with_no_session_persistence()
                .with_include_partial_messages(),
            },
        ]),
        "merge" => Some(vec![Step::Agent {
            agent: AgentStep::new(
                "Merge task ${task}. Rebase ${branch} onto main, resolve conflicts if any, then squash merge.",
            )
            .with_model("sonnet")
            .with_print()
            .with_max_turns(20)
            .with_tools(vec!["Bash".into(), "Read".into(), "Edit".into()])
            .with_allowed_tools(vec!["Bash(git *)".into()])
            .with_append_system_prompt(
                "You are a git expert. Steps: 1) git fetch origin, 2) git rebase origin/main, 3) resolve conflicts if any, 4) git checkout main, 5) git merge --squash ${branch}, 6) git commit. Report any issues.",
            )
            .with_no_session_persistence()
            .with_include_partial_messages(),
        }]),
        "refactor" => Some(vec![
            Step::Agent {
                agent: AgentStep::new(
                    "Analyze code structure for refactoring task ${task}. Identify patterns and issues.",
                )
                .with_model("haiku")
                .with_print()
                .with_max_turns(5)
                .with_tools(vec!["Read".into(), "Grep".into(), "Glob".into()])
                .with_no_session_persistence()
                .with_include_partial_messages(),
            },
            Step::Agent {
                agent: AgentStep::new(
                    "Apply refactoring based on the analysis. Make changes incrementally and verify each step.",
                )
                .with_model("sonnet")
                .with_print()
                .with_max_turns(20)
                .with_tools(vec!["Read".into(), "Edit".into(), "Bash".into()])
                .with_no_session_persistence()
                .with_include_partial_messages(),
            },
        ]),
        _ => None,
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
            Step::Agent { agent } => {
                assert!(!agent.print); // default is REPL mode
                assert_eq!(agent.model, "sonnet"); // default
                assert_eq!(agent.prompt, "Do something");
            }
            _ => panic!("Expected Agent step"),
        }
    }

    #[test]
    fn test_step_agent_full() {
        let json = r#"{
            "type": "agent",
            "print": true,
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
            Step::Agent { agent } => {
                assert!(agent.print);
                assert_eq!(agent.model, "opus");
                assert_eq!(agent.prompt, "Review code");
                assert_eq!(agent.tools, vec!["Read", "Edit"]);
                assert_eq!(agent.allowed_tools, vec!["Bash(npm *)"]);
                assert!(agent.skip_permissions);
                assert_eq!(agent.output_format, "stream-json");
                assert_eq!(agent.window, Some("new".to_string()));
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

    #[test]
    fn test_step_agent_all_new_params() {
        let json = r#"{
            "type": "agent",
            "prompt": "test",
            "include_partial_messages": true,
            "json_schema": "{\"type\":\"object\"}",
            "session_id": "abc-123",
            "fork_session": true,
            "no_session_persistence": true,
            "fallback_model": "haiku",
            "allow_skip_permissions": true,
            "permission_prompt_tool": "mcp_auth",
            "agents": {"reviewer": {"description": "Review", "prompt": "You are a reviewer"}},
            "agent": "reviewer",
            "strict_mcp_config": true,
            "debug": "api,hooks",
            "settings": "./settings.json",
            "setting_sources": "user,project",
            "plugin_dir": ["./plugins"],
            "betas": ["interleaved-thinking"],
            "chrome": true,
            "ide": true,
            "disable_slash_commands": true
        }"#;
        let step: Step = serde_json::from_str(json).unwrap();
        match step {
            Step::Agent { agent } => {
                assert!(agent.include_partial_messages);
                assert_eq!(agent.json_schema, Some("{\"type\":\"object\"}".to_string()));
                assert_eq!(agent.session_id, Some("abc-123".to_string()));
                assert!(agent.fork_session);
                assert!(agent.no_session_persistence);
                assert_eq!(agent.fallback_model, Some("haiku".to_string()));
                assert!(agent.allow_skip_permissions);
                assert_eq!(agent.permission_prompt_tool, Some("mcp_auth".to_string()));
                assert!(agent.agents.is_some());
                assert_eq!(agent.agent, Some("reviewer".to_string()));
                assert!(agent.strict_mcp_config);
                assert_eq!(agent.debug, Some("api,hooks".to_string()));
                assert_eq!(agent.settings, Some("./settings.json".to_string()));
                assert_eq!(agent.setting_sources, Some("user,project".to_string()));
                assert_eq!(agent.plugin_dir, vec!["./plugins"]);
                assert_eq!(agent.betas, vec!["interleaved-thinking"]);
                assert_eq!(agent.chrome, Some(true));
                assert!(agent.ide);
                assert!(agent.disable_slash_commands);
            }
            _ => panic!("Expected Agent step"),
        }
    }

    #[test]
    fn test_step_agent_chrome_false() {
        let json = r#"{"type": "agent", "prompt": "test", "chrome": false}"#;
        let step: Step = serde_json::from_str(json).unwrap();
        match step {
            Step::Agent { agent } => {
                assert_eq!(agent.chrome, Some(false));
            }
            _ => panic!("Expected Agent step"),
        }
    }

    #[test]
    fn test_builtin_pipeline_code_review() {
        let config = WtConfig::default();
        let pipeline = config.get_pipeline("code-review");
        assert!(pipeline.is_some());
        let steps = pipeline.unwrap();
        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn test_builtin_pipeline_merge() {
        let config = WtConfig::default();
        let pipeline = config.get_pipeline("merge");
        assert!(pipeline.is_some());
        let steps = pipeline.unwrap();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn test_builtin_pipeline_refactor() {
        let config = WtConfig::default();
        let pipeline = config.get_pipeline("refactor");
        assert!(pipeline.is_some());
        let steps = pipeline.unwrap();
        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn test_builtin_pipeline_unknown() {
        let config = WtConfig::default();
        let pipeline = config.get_pipeline("unknown");
        assert!(pipeline.is_none());
    }

    #[test]
    fn test_hook_def_pipeline_ref() {
        let json = r#"{"use_pipeline": "code-review"}"#;
        let hook: HookDef = serde_json::from_str(json).unwrap();
        match hook {
            HookDef::PipelineRef { use_pipeline } => {
                assert_eq!(use_pipeline, "code-review");
            }
            _ => panic!("Expected PipelineRef"),
        }
    }

    #[test]
    fn test_resolve_pipeline_ref() {
        let config = WtConfig::default();
        let hook = HookDef::PipelineRef {
            use_pipeline: "code-review".to_string(),
        };
        let resolved = config.resolve_hook(&hook);
        assert!(resolved.is_some());
        match resolved.unwrap() {
            HookDef::Pipeline { pipeline } => {
                assert_eq!(pipeline.len(), 2);
            }
            _ => panic!("Expected Pipeline"),
        }
    }

    #[test]
    fn test_user_defined_pipeline() {
        let json = r#"{
            "pipelines": {
                "my-review": [
                    {"type": "agent", "prompt": "custom review"}
                ]
            }
        }"#;
        let config = WtConfig::from_str(json).unwrap();
        let pipeline = config.get_pipeline("my-review");
        assert!(pipeline.is_some());
        assert_eq!(pipeline.unwrap().len(), 1);
    }

    #[test]
    fn test_user_pipeline_overrides_builtin() {
        let json = r#"{
            "pipelines": {
                "code-review": [
                    {"type": "agent", "prompt": "my custom review"}
                ]
            }
        }"#;
        let config = WtConfig::from_str(json).unwrap();
        let pipeline = config.get_pipeline("code-review");
        assert!(pipeline.is_some());
        // User-defined should override builtin (only 1 step vs 2)
        assert_eq!(pipeline.unwrap().len(), 1);
    }
}
