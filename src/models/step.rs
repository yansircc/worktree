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
use std::time::Duration;

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
    /// Retry attempt number (0 = first attempt)
    #[serde(default)]
    pub attempt: u32,
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
            attempt: 0,
        }
    }
}

impl StepResult {
    /// Create a result with a specific attempt number
    pub fn with_attempt(mut self, attempt: u32) -> Self {
        self.attempt = attempt;
        self
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

impl VerifyAction {
    fn failed_default() -> Self {
        VerifyAction::Failed
    }
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

// ============================================================================
// OnError
// ============================================================================

/// Step failure handling action
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OnError {
    /// Use workflow-level setting (default)
    #[default]
    Inherit,
    /// Mark as failed, abort workflow
    Fail,
    /// Mark as failed, continue execution
    Continue,
    /// Mark as blocked, pause workflow
    Block,
    /// Retry the step (uses step.retry config)
    Retry,
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

/// Parse a duration string (e.g., "30s", "5m", "1h", "100ms")
///
/// Supported formats:
/// - `Nms` - milliseconds
/// - `Ns` - seconds
/// - `Nm` - minutes
/// - `Nh` - hours
///
/// Returns None if parsing fails.
pub fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    // Try each suffix in order (longest first to avoid ambiguity)
    if let Some(num) = s.strip_suffix("ms") {
        num.parse::<u64>().ok().map(Duration::from_millis)
    } else if let Some(num) = s.strip_suffix('s') {
        num.parse::<u64>().ok().map(Duration::from_secs)
    } else if let Some(num) = s.strip_suffix('m') {
        num.parse::<u64>().ok().map(|n| Duration::from_secs(n * 60))
    } else if let Some(num) = s.strip_suffix('h') {
        num.parse::<u64>().ok().map(|n| Duration::from_secs(n * 3600))
    } else {
        // Try parsing as seconds if no suffix
        s.parse::<u64>().ok().map(Duration::from_secs)
    }
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

    /// Error handling action
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_error: Option<OnError>,

    /// Dependencies (for DAG mode)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends: Vec<String>,
}

impl Step {
    // Test helper: Create a simple script step
    #[cfg(test)]
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
            on_error: None,
            depends: Vec::new(),
        }
    }

    /// Get effective on_error, considering inheritance
    pub fn effective_on_error(&self) -> &OnError {
        self.on_error.as_ref().unwrap_or(&OnError::Inherit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_on_error_default() {
        assert_eq!(OnError::default(), OnError::Inherit);
    }

    #[test]
    fn test_on_error_serialize_deserialize() {
        let json = r#"{"id": "test", "run": "echo test", "on_error": "continue"}"#;
        let step: Step = serde_json::from_str(json).unwrap();
        assert_eq!(step.on_error, Some(OnError::Continue));

        let json = r#"{"id": "test", "run": "echo test", "on_error": "retry"}"#;
        let step: Step = serde_json::from_str(json).unwrap();
        assert_eq!(step.on_error, Some(OnError::Retry));
    }

    #[test]
    fn test_effective_on_error() {
        let step = Step::script("echo test");
        assert_eq!(step.effective_on_error(), &OnError::Inherit);

        let mut step = Step::script("echo test");
        step.on_error = Some(OnError::Continue);
        assert_eq!(step.effective_on_error(), &OnError::Continue);
    }

    #[test]
    fn test_parse_duration_seconds() {
        assert_eq!(parse_duration("30s"), Some(Duration::from_secs(30)));
        assert_eq!(parse_duration("1s"), Some(Duration::from_secs(1)));
        assert_eq!(parse_duration("0s"), Some(Duration::from_secs(0)));
    }

    #[test]
    fn test_parse_duration_minutes() {
        assert_eq!(parse_duration("5m"), Some(Duration::from_secs(5 * 60)));
        assert_eq!(parse_duration("1m"), Some(Duration::from_secs(60)));
    }

    #[test]
    fn test_parse_duration_hours() {
        assert_eq!(parse_duration("1h"), Some(Duration::from_secs(3600)));
        assert_eq!(parse_duration("2h"), Some(Duration::from_secs(7200)));
    }

    #[test]
    fn test_parse_duration_milliseconds() {
        assert_eq!(parse_duration("100ms"), Some(Duration::from_millis(100)));
        assert_eq!(parse_duration("500ms"), Some(Duration::from_millis(500)));
    }

    #[test]
    fn test_parse_duration_no_suffix() {
        assert_eq!(parse_duration("30"), Some(Duration::from_secs(30)));
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert_eq!(parse_duration(""), None);
        assert_eq!(parse_duration("abc"), None);
        assert_eq!(parse_duration("10x"), None);
    }

    #[test]
    fn test_step_result_with_attempt() {
        let result = StepResult::default().with_attempt(2);
        assert_eq!(result.attempt, 2);
    }
}
