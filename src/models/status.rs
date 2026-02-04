//! Status types for task management.
//!
//! - TaskStatus: Pending / Active / Idle / Completed
//! - StepResult: Done / HumanReview / Error / Conflict / Timeout / Manual
//! - TaskState: Runtime state for a single task

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

use crate::models::Instance;

// ============================================================================
// Task Status (reflects whether a process is running)
// ============================================================================

/// Task status reflecting resource state
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// Task defined but resources not created (no worktree, no branch)
    #[default]
    Pending,
    /// A process is running (agent, script, or pipeline)
    Active,
    /// Resources exist but no process running
    Idle,
    /// Task completed (worktree deleted, branch merged)
    Completed,
}

impl TaskStatus {
    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Active => "active",
            TaskStatus::Idle => "idle",
            TaskStatus::Completed => "completed",
        }
    }

    /// Get status icon
    pub fn icon(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "○",
            TaskStatus::Active => "●",
            TaskStatus::Idle => "◐",
            TaskStatus::Completed => "✓",
        }
    }

    /// Get colored status icon for terminal display
    pub fn colored_icon(&self) -> String {
        use crate::display::{GREEN, MAGENTA, RESET, WHITE, YELLOW};

        let color = match self {
            TaskStatus::Pending => WHITE,
            TaskStatus::Active => GREEN,
            TaskStatus::Idle => YELLOW,
            TaskStatus::Completed => MAGENTA,
        };
        format!("{}{}{}", color, self.icon(), RESET)
    }
}

// ============================================================================
// Idle Reason
// ============================================================================

/// Reason for being in Idle state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepResult {
    /// Current phase completed normally, waiting for next step
    Done,
    /// Waiting for human review
    HumanReview,
    /// Command/agent execution error (including crashes)
    Error,
    /// Merge conflict needs resolution
    Conflict,
    /// Execution timeout
    Timeout,
    /// User manually paused
    Manual,
}

impl StepResult {
    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            StepResult::Done => "done",
            StepResult::HumanReview => "human_review",
            StepResult::Error => "error",
            StepResult::Conflict => "conflict",
            StepResult::Timeout => "timeout",
            StepResult::Manual => "manual",
        }
    }
}

// ============================================================================
// Task State
// ============================================================================

/// Deserialize phase with backward compatibility.
/// Converts "none" to None for compatibility with old status.json files.
fn deserialize_phase<'de, D>(deserializer: D) -> std::result::Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    Ok(opt.filter(|s| s != "none"))
}

/// Runtime state for a single task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskState {
    /// Current status (process state)
    pub status: TaskStatus,

    /// Current phase (business progress) - arbitrary string from config sequence
    /// None means task is in pending state (before first phase)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_phase")]
    pub phase: Option<String>,

    /// Reason for being idle (only when status is Idle)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_result: Option<StepResult>,

    /// Timestamp when entered Active state (for monitoring)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_since: Option<DateTime<Utc>>,

    /// Instance information (worktree, branch, session)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance: Option<Instance>,

    /// Scratch environment flag
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scratch: Option<bool>,
}

impl Default for TaskState {
    fn default() -> Self {
        Self {
            status: TaskStatus::Pending,
            phase: None,
            step_result: None,
            active_since: None,
            instance: None,
            scratch: None,
        }
    }
}

impl TaskState {
    /// Transition to Idle state
    pub fn to_idle(&mut self, reason: StepResult) {
        self.status = TaskStatus::Idle;
        self.step_result = Some(reason);
        self.active_since = None;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== TaskStatus Tests ====================

    #[test]
    fn test_status_default() {
        let status: TaskStatus = Default::default();
        assert_eq!(status, TaskStatus::Pending);
    }

    #[test]
    fn test_status_serialize() {
        assert_eq!(
            serde_json::to_string(&TaskStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&TaskStatus::Active).unwrap(),
            "\"active\""
        );
        assert_eq!(
            serde_json::to_string(&TaskStatus::Idle).unwrap(),
            "\"idle\""
        );
        assert_eq!(
            serde_json::to_string(&TaskStatus::Completed).unwrap(),
            "\"completed\""
        );
    }

    #[test]
    fn test_status_deserialize() {
        assert_eq!(
            serde_json::from_str::<TaskStatus>("\"pending\"").unwrap(),
            TaskStatus::Pending
        );
        assert_eq!(
            serde_json::from_str::<TaskStatus>("\"active\"").unwrap(),
            TaskStatus::Active
        );
        assert_eq!(
            serde_json::from_str::<TaskStatus>("\"idle\"").unwrap(),
            TaskStatus::Idle
        );
        assert_eq!(
            serde_json::from_str::<TaskStatus>("\"completed\"").unwrap(),
            TaskStatus::Completed
        );
    }

    // ==================== StepResult Tests ====================

    #[test]
    fn test_step_result_serialize() {
        assert_eq!(
            serde_json::to_string(&StepResult::Done).unwrap(),
            "\"done\""
        );
        assert_eq!(
            serde_json::to_string(&StepResult::HumanReview).unwrap(),
            "\"human_review\""
        );
        assert_eq!(
            serde_json::to_string(&StepResult::Error).unwrap(),
            "\"error\""
        );
        assert_eq!(
            serde_json::to_string(&StepResult::Conflict).unwrap(),
            "\"conflict\""
        );
        assert_eq!(
            serde_json::to_string(&StepResult::Timeout).unwrap(),
            "\"timeout\""
        );
        assert_eq!(
            serde_json::to_string(&StepResult::Manual).unwrap(),
            "\"manual\""
        );
    }

    // ==================== TaskState Tests ====================

    #[test]
    fn test_state_default() {
        let state = TaskState::default();
        assert_eq!(state.status, TaskStatus::Pending);
        assert!(state.phase.is_none());
        assert!(state.step_result.is_none());
        assert!(state.active_since.is_none());
        assert!(state.instance.is_none());
    }

    #[test]
    fn test_state_to_idle() {
        let mut state = TaskState::default();
        state.status = TaskStatus::Active;
        state.phase = Some("developing".to_string());
        state.active_since = Some(Utc::now());

        state.to_idle(StepResult::Done);
        assert_eq!(state.status, TaskStatus::Idle);
        assert_eq!(state.step_result, Some(StepResult::Done));
        assert!(state.active_since.is_none());
    }

    #[test]
    fn test_state_serialize_minimal() {
        let state = TaskState::default();
        let json = serde_json::to_string(&state).unwrap();

        // Should only have status, phase=None should be skipped
        assert!(json.contains("\"status\":\"pending\""));
        assert!(!json.contains("phase"));
        assert!(!json.contains("step_result"));
        assert!(!json.contains("active_since"));
    }

    #[test]
    fn test_state_serialize_full() {
        let mut state = TaskState::default();
        state.status = TaskStatus::Active;
        state.phase = Some("developing".to_string());
        state.active_since = Some(Utc::now());
        state.instance = Some(Instance {
            branch: Some("wt/test".to_string()),
            worktree_path: Some("/path".to_string()),
            session_name: "wt".to_string(),
            window_name: Some("test".to_string()),
            session_id: None,
            multiplexer: crate::services::multiplexer::MultiplexerType::Tmux,
        });

        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("\"status\":\"active\""));
        assert!(json.contains("\"phase\":\"developing\""));
        assert!(json.contains("\"active_since\""));
        assert!(json.contains("\"instance\""));
    }

    #[test]
    fn test_state_deserialize_backward_compat() {
        // Old format without phase
        let json = r#"{"status":"pending"}"#;
        let state: TaskState = serde_json::from_str(json).unwrap();
        assert_eq!(state.status, TaskStatus::Pending);
        assert!(state.phase.is_none());
    }

    #[test]
    fn test_state_deserialize_legacy_none_phase() {
        // Old format with "none" phase should be converted to None
        let json = r#"{"status":"pending","phase":"none"}"#;
        let state: TaskState = serde_json::from_str(json).unwrap();
        assert!(state.phase.is_none());
    }

    #[test]
    fn test_state_deserialize_arbitrary_phase() {
        // New format with arbitrary phase name
        let json = r#"{"status":"active","phase":"custom-phase"}"#;
        let state: TaskState = serde_json::from_str(json).unwrap();
        assert_eq!(state.phase, Some("custom-phase".to_string()));
    }
}
