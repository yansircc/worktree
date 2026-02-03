//! Agent step configuration structure.
//!
//! Separates agent configuration from the Step enum for better ergonomics.

use serde::{Deserialize, Serialize};

/// Agent step configuration - all fields have sensible defaults
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentStep {
    // === Basic ===
    /// Prompt text or @file reference
    pub prompt: String,
    /// Print mode - non-interactive (-p / --print)
    #[serde(default)]
    pub print: bool,
    /// Model: haiku, sonnet, opus, or full model name (--model)
    #[serde(default = "default_model")]
    pub model: String,

    // === System Prompt ===
    /// Replace entire system prompt (--system-prompt)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Load system prompt from file (--system-prompt-file)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt_file: Option<String>,
    /// Append to system prompt (--append-system-prompt)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub append_system_prompt: Option<String>,
    /// Append system prompt from file (--append-system-prompt-file)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub append_system_prompt_file: Option<String>,

    // === Tools ===
    /// Restrict available tools (--tools)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    /// Auto-approve tools (--allowedTools)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,
    /// Disable tools (--disallowedTools)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disallowed_tools: Vec<String>,

    // === Permissions ===
    /// Skip all permission prompts (--dangerously-skip-permissions)
    #[serde(default)]
    pub skip_permissions: bool,
    /// Enable skip permissions as option without activating (--allow-dangerously-skip-permissions)
    #[serde(default)]
    pub allow_skip_permissions: bool,
    /// Permission mode: plan, etc. (--permission-mode)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    /// MCP tool for permission prompts (--permission-prompt-tool)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_prompt_tool: Option<String>,

    // === Limits ===
    /// Max agentic turns (--max-turns)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
    /// Max budget in USD (--max-budget-usd)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_budget_usd: Option<f64>,

    // === Session ===
    /// Continue most recent conversation (--continue)
    #[serde(default, rename = "continue")]
    pub continue_session: bool,
    /// Resume specific session by ID or name (--resume)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume: Option<String>,
    /// Specific session ID (--session-id)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Fork session instead of reusing (--fork-session)
    #[serde(default)]
    pub fork_session: bool,
    /// Disable session persistence (--no-session-persistence)
    #[serde(default)]
    pub no_session_persistence: bool,

    // === Input/Output ===
    /// Output format: text, json, stream-json (--output-format)
    #[serde(default = "default_output_format")]
    pub output_format: String,
    /// Input format: text, stream-json (--input-format)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_format: Option<String>,
    /// Include partial streaming events (--include-partial-messages)
    #[serde(default)]
    pub include_partial_messages: bool,
    /// JSON schema for structured output (--json-schema)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<String>,

    // === Model (additional) ===
    /// Fallback model when primary is overloaded (--fallback-model)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_model: Option<String>,

    // === Subagents ===
    /// Custom subagents definition (--agents)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents: Option<serde_json::Value>,
    /// Specify an agent for the session (--agent)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,

    // === Other ===
    /// Additional working directories (--add-dir)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub add_dir: Vec<String>,
    /// MCP config file or JSON (--mcp-config)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_config: Option<String>,
    /// Only use MCP servers from mcp_config (--strict-mcp-config)
    #[serde(default)]
    pub strict_mcp_config: bool,
    /// Enable verbose logging (--verbose)
    #[serde(default)]
    pub verbose: bool,
    /// Debug mode with optional category filtering (--debug)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debug: Option<String>,
    /// Settings file path or JSON (--settings)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<String>,
    /// Setting sources to load (--setting-sources)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setting_sources: Option<String>,
    /// Plugin directories (--plugin-dir)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugin_dir: Vec<String>,
    /// Beta features (--betas)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub betas: Vec<String>,

    // === Browser/IDE ===
    /// Chrome browser integration (--chrome / --no-chrome)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chrome: Option<bool>,
    /// Auto-connect to IDE (--ide)
    #[serde(default)]
    pub ide: bool,
    /// Disable slash commands (--disable-slash-commands)
    #[serde(default)]
    pub disable_slash_commands: bool,

    // === Window (for interactive mode) ===
    /// Window mode: main, new
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<String>,
}

fn default_model() -> String {
    "sonnet".to_string()
}

fn default_output_format() -> String {
    "text".to_string()
}

impl Default for AgentStep {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            print: false,
            model: default_model(),
            system_prompt: None,
            system_prompt_file: None,
            append_system_prompt: None,
            append_system_prompt_file: None,
            tools: Vec::new(),
            allowed_tools: Vec::new(),
            disallowed_tools: Vec::new(),
            skip_permissions: false,
            allow_skip_permissions: false,
            permission_mode: None,
            permission_prompt_tool: None,
            max_turns: None,
            max_budget_usd: None,
            continue_session: false,
            resume: None,
            session_id: None,
            fork_session: false,
            no_session_persistence: false,
            output_format: default_output_format(),
            input_format: None,
            include_partial_messages: false,
            json_schema: None,
            fallback_model: None,
            agents: None,
            agent: None,
            add_dir: Vec::new(),
            mcp_config: None,
            strict_mcp_config: false,
            verbose: false,
            debug: None,
            settings: None,
            setting_sources: None,
            plugin_dir: Vec::new(),
            betas: Vec::new(),
            chrome: None,
            ide: false,
            disable_slash_commands: false,
            window: None,
        }
    }
}

impl AgentStep {
    /// Create a new agent step with just a prompt
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            ..Default::default()
        }
    }

    /// Builder-style setter for model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Builder-style setter for print mode
    pub fn with_print(mut self) -> Self {
        self.print = true;
        self
    }

    /// Builder-style setter for max_turns
    pub fn with_max_turns(mut self, turns: u32) -> Self {
        self.max_turns = Some(turns);
        self
    }

    /// Builder-style setter for tools
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tools = tools;
        self
    }

    /// Builder-style setter for allowed_tools
    pub fn with_allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.allowed_tools = tools;
        self
    }

    /// Builder-style setter for append_system_prompt
    pub fn with_append_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.append_system_prompt = Some(prompt.into());
        self
    }

    /// Builder-style setter for no_session_persistence
    pub fn with_no_session_persistence(mut self) -> Self {
        self.no_session_persistence = true;
        self
    }

    /// Builder-style setter for include_partial_messages
    pub fn with_include_partial_messages(mut self) -> Self {
        self.include_partial_messages = true;
        self
    }

    /// Create a default development agent for a task
    pub fn default_develop(task_name: &str) -> Self {
        Self::new(format!(
            "@.wt/tasks/{}.md 请完成这个任务。完成后运行 `wt step done` 标记完成。",
            task_name
        ))
    }

    /// Create a default review agent for a task
    pub fn default_review(task_name: &str) -> Self {
        Self::new(format!(
            "请 review 任务 {} 的代码变更。完成后运行 `wt step done` 标记完成。",
            task_name
        ))
        .with_model("opus")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_step_new() {
        let step = AgentStep::new("Do something");
        assert_eq!(step.prompt, "Do something");
        assert_eq!(step.model, "sonnet");
        assert!(!step.print);
    }

    #[test]
    fn test_agent_step_builder() {
        let step = AgentStep::new("Review code")
            .with_model("opus")
            .with_print()
            .with_max_turns(10)
            .with_tools(vec!["Read".into(), "Edit".into()]);

        assert_eq!(step.prompt, "Review code");
        assert_eq!(step.model, "opus");
        assert!(step.print);
        assert_eq!(step.max_turns, Some(10));
        assert_eq!(step.tools, vec!["Read", "Edit"]);
    }

    #[test]
    fn test_agent_step_default() {
        let step = AgentStep::default();
        assert!(step.prompt.is_empty());
        assert_eq!(step.model, "sonnet");
        assert_eq!(step.output_format, "text");
    }

    #[test]
    fn test_agent_step_serde() {
        let json = r#"{"prompt": "test", "print": true, "model": "haiku"}"#;
        let step: AgentStep = serde_json::from_str(json).unwrap();
        assert_eq!(step.prompt, "test");
        assert!(step.print);
        assert_eq!(step.model, "haiku");
    }

    #[test]
    fn test_agent_step_serde_minimal() {
        let json = r#"{"prompt": "minimal test"}"#;
        let step: AgentStep = serde_json::from_str(json).unwrap();
        assert_eq!(step.prompt, "minimal test");
        assert_eq!(step.model, "sonnet"); // default
        assert!(!step.print); // default
    }

    #[test]
    fn test_agent_step_serde_roundtrip() {
        let step = AgentStep::new("Test prompt")
            .with_model("opus")
            .with_print()
            .with_max_turns(5);

        let json = serde_json::to_string(&step).unwrap();
        let parsed: AgentStep = serde_json::from_str(&json).unwrap();

        assert_eq!(step, parsed);
    }
}
