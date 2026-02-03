//! Step model for Phases v2 system.
//!
//! Step is the smallest execution unit with 5 orthogonal dimensions:
//! - execute: what to run (script or agent)
//! - input: environment, files, context, stdin
//! - output: artifacts, exports
//! - observe: terminal, log
//! - verify: self, script, agent, human, schema

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::models::AgentStep;

// ============================================================================
// StepState
// ============================================================================

/// Step execution state
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StepState {
    /// Waiting to execute
    #[default]
    Pending,
    /// Currently executing
    Running,
    /// Execution succeeded
    Success,
    /// Execution failed
    Failed,
    /// Needs intervention (Agent marked via `wt step block`)
    Blocked,
    /// Execution timed out
    Timeout,
    /// Skipped (condition not met)
    Skipped,
}

impl StepState {
    /// Check if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            StepState::Success | StepState::Failed | StepState::Blocked | StepState::Timeout | StepState::Skipped
        )
    }

    /// Check if this represents a successful outcome
    pub fn is_success(&self) -> bool {
        matches!(self, StepState::Success | StepState::Skipped)
    }

    /// Get display icon
    pub fn icon(&self) -> &'static str {
        match self {
            StepState::Pending => "○",
            StepState::Running => "●",
            StepState::Success => "✓",
            StepState::Failed => "✗",
            StepState::Blocked => "⊘",
            StepState::Timeout => "⏱",
            StepState::Skipped => "⊖",
        }
    }
}

// ============================================================================
// StepResult
// ============================================================================

/// Result of step execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Step identifier (if named)
    pub step_id: Option<String>,
    /// Final state
    pub state: StepState,
    /// Process exit code (for script steps)
    pub exit_code: Option<i32>,
    /// Message (reason for blocked/failed)
    pub message: Option<String>,
    /// Path to output log file
    pub output_file: PathBuf,
    /// Collected artifacts
    pub artifacts: Vec<PathBuf>,
    /// Exported variables
    pub exports: HashMap<String, String>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
}

impl Default for StepResult {
    fn default() -> Self {
        Self {
            step_id: None,
            state: StepState::Pending,
            exit_code: None,
            message: None,
            output_file: PathBuf::new(),
            artifacts: Vec::new(),
            exports: HashMap::new(),
            duration_ms: 0,
        }
    }
}

// ============================================================================
// Step Input
// ============================================================================

/// Input configuration for a step
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StepInput {
    /// Environment variables
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
    /// Files to read (paths with variable expansion)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    /// Context from previous steps (e.g., "${steps.analyze.output}")
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<String>,
    /// Stdin from previous step (e.g., "${prev.stdout}")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdin: Option<String>,
}

// ============================================================================
// Step Output
// ============================================================================

/// Output configuration for a step
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StepOutput {
    /// Artifact patterns to collect (e.g., "dist/**", "coverage/")
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<String>,
    /// Variables to export (name -> shell command to extract value)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub export: HashMap<String, String>,
}

// ============================================================================
// Step Observe
// ============================================================================

/// Observation mode
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ObserveMode {
    /// Interactive mode (foreground, with terminal)
    #[default]
    Interactive,
    /// Background mode (no terminal interaction)
    Background,
}

/// Output target
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputTarget {
    /// Terminal only
    Terminal,
    /// File only
    File,
    /// Both terminal and file
    #[default]
    Both,
}

/// Multiplexer observation settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MultiplexerObserve {
    /// Window name (default: task name)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<String>,
    /// Whether to focus this window
    #[serde(default)]
    pub focus: bool,
}

/// Log observation settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogObserve {
    /// Log file path (supports variable expansion)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Whether to stream output in real-time
    #[serde(default)]
    pub stream: bool,
}

/// Observation configuration for a step
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StepObserve {
    /// Observation mode
    #[serde(default)]
    pub mode: ObserveMode,
    /// Output target
    #[serde(default)]
    pub output: OutputTarget,
    /// Multiplexer settings
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multiplexer: Option<MultiplexerObserve>,
    /// Log settings
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log: Option<LogObserve>,
}

// ============================================================================
// Step Verify
// ============================================================================

/// Verification type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VerifyType {
    /// Agent self-marks via `wt step done/block/fail`
    #[serde(rename = "self")]
    SelfMark,
    /// Run a script to verify
    Script,
    /// Use an agent to verify
    Agent,
    /// Require human verification
    Human,
    /// Validate output against JSON schema
    Schema,
}

impl Default for VerifyType {
    fn default() -> Self {
        VerifyType::SelfMark
    }
}

/// Action on verification result
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VerifyAction {
    /// Mark step as success
    #[default]
    Success,
    /// Mark step as failed
    Failed,
    /// Mark step as blocked
    Blocked,
    /// Retry the step
    Retry,
}

/// Verification configuration for a step
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StepVerify {
    /// Agent self-marks
    #[serde(rename = "self")]
    SelfMark,
    /// Script verification
    Script {
        /// Script to run
        run: String,
        /// Action on pass
        #[serde(default)]
        on_pass: VerifyAction,
        /// Action on fail
        #[serde(default = "VerifyAction::failed_default")]
        on_fail: VerifyAction,
    },
    /// Agent verification
    Agent {
        /// Agent configuration
        agent: AgentStep,
    },
    /// Human verification
    Human {
        /// Prompt to display
        prompt: String,
        /// Timeout duration
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout: Option<String>,
    },
    /// Schema verification
    Schema {
        /// JSON schema to validate against
        schema: String,
    },
}

impl Default for StepVerify {
    fn default() -> Self {
        StepVerify::SelfMark
    }
}

impl VerifyAction {
    fn failed_default() -> Self {
        VerifyAction::Failed
    }
}

// ============================================================================
// Step Retry
// ============================================================================

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRetry {
    /// Maximum retry attempts
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    /// Delay between retries (e.g., "30s", "1m")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay: Option<String>,
}

fn default_max_attempts() -> u32 {
    2
}

impl Default for StepRetry {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
            delay: None,
        }
    }
}

// ============================================================================
// Step Execute
// ============================================================================

/// Step executor - either a script or an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StepExecute {
    /// Run a shell script
    Script {
        /// Shell command to run
        run: String,
    },
    /// Run a Claude agent
    Agent {
        /// Agent configuration
        agent: AgentStep,
    },
}

// ============================================================================
// Step (main struct)
// ============================================================================

/// A single execution step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    /// Step identifier (optional, for referencing)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Human-readable name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    // ========== Execute ==========
    /// Shell command to run (mutually exclusive with `agent`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,

    /// Agent configuration (mutually exclusive with `run`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentStep>,

    // ========== Input ==========
    /// Input configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<StepInput>,

    // ========== Output ==========
    /// Output configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<StepOutput>,

    // ========== Observe ==========
    /// Observation configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observe: Option<StepObserve>,

    // ========== Verify ==========
    /// Verification configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verify: Option<StepVerify>,

    // ========== Control ==========
    /// Condition for execution (variable expression)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,

    /// Timeout duration (e.g., "30m", "1h")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,

    /// Retry configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<StepRetry>,

    /// Dependencies (for DAG mode)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends: Vec<String>,
}

impl Step {
    /// Create a simple script step
    pub fn script(command: impl Into<String>) -> Self {
        Self {
            id: None,
            name: None,
            run: Some(command.into()),
            agent: None,
            input: None,
            output: None,
            observe: None,
            verify: None,
            condition: None,
            timeout: None,
            retry: None,
            depends: Vec::new(),
        }
    }

    /// Create an agent step
    pub fn agent(agent_config: AgentStep) -> Self {
        Self {
            id: None,
            name: None,
            run: None,
            agent: Some(agent_config),
            input: None,
            output: None,
            observe: None,
            verify: None,
            condition: None,
            timeout: None,
            retry: None,
            depends: Vec::new(),
        }
    }

    /// Check if this is a script step
    pub fn is_script(&self) -> bool {
        self.run.is_some()
    }

    /// Check if this is an agent step
    pub fn is_agent(&self) -> bool {
        self.agent.is_some()
    }

    /// Get step display name
    pub fn display_name(&self) -> String {
        if let Some(name) = &self.name {
            name.clone()
        } else if let Some(id) = &self.id {
            id.clone()
        } else if let Some(run) = &self.run {
            // Truncate long commands
            if run.len() > 30 {
                format!("{}...", &run[..27])
            } else {
                run.clone()
            }
        } else if self.agent.is_some() {
            "agent".to_string()
        } else {
            "step".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_state_is_terminal() {
        assert!(!StepState::Pending.is_terminal());
        assert!(!StepState::Running.is_terminal());
        assert!(StepState::Success.is_terminal());
        assert!(StepState::Failed.is_terminal());
        assert!(StepState::Blocked.is_terminal());
        assert!(StepState::Timeout.is_terminal());
        assert!(StepState::Skipped.is_terminal());
    }

    #[test]
    fn test_step_state_is_success() {
        assert!(!StepState::Pending.is_success());
        assert!(!StepState::Running.is_success());
        assert!(StepState::Success.is_success());
        assert!(!StepState::Failed.is_success());
        assert!(!StepState::Blocked.is_success());
        assert!(StepState::Skipped.is_success());
    }

    #[test]
    fn test_step_script_creation() {
        let step = Step::script("npm test");
        assert!(step.is_script());
        assert!(!step.is_agent());
        assert_eq!(step.run, Some("npm test".to_string()));
    }

    #[test]
    fn test_step_display_name() {
        let mut step = Step::script("npm test");
        assert_eq!(step.display_name(), "npm test");

        step.name = Some("Run tests".to_string());
        assert_eq!(step.display_name(), "Run tests");

        step.name = None;
        step.id = Some("test".to_string());
        assert_eq!(step.display_name(), "test");
    }

    #[test]
    fn test_step_serialize_deserialize() {
        let json = r#"{
            "id": "test",
            "run": "npm test",
            "condition": "${prev.state} == 'success'"
        }"#;

        let step: Step = serde_json::from_str(json).unwrap();
        assert_eq!(step.id, Some("test".to_string()));
        assert_eq!(step.run, Some("npm test".to_string()));
        assert!(step.condition.is_some());
    }

    #[test]
    fn test_verify_serialize() {
        let verify = StepVerify::Script {
            run: "npm test".to_string(),
            on_pass: VerifyAction::Success,
            on_fail: VerifyAction::Failed,
        };
        let json = serde_json::to_string(&verify).unwrap();
        assert!(json.contains("script"));
        assert!(json.contains("npm test"));
    }
}
