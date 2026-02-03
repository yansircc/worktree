//! Status model for wt task management.
//!
//! - Status: Pending / Active / Idle / Completed
//! - Phase: None / Developing / Reviewing / Merging
//! - IdleReason: Done / HumanReview / Error / Conflict / Timeout / Manual

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Result, WtError};
use crate::models::Instance;

/// Path to the v2 status file
pub const STATUS_FILE: &str = ".wt/status.json";

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
// Task Phase (reflects business progress)
// ============================================================================

/// Task phase reflecting business progress
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskPhase {
    /// Not started
    #[default]
    #[serde(rename = "none")]
    None,
    /// Development in progress
    Developing,
    /// Code review in progress
    Reviewing,
    /// Merge in progress
    Merging,
}

impl TaskPhase {
    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            TaskPhase::None => "none",
            TaskPhase::Developing => "developing",
            TaskPhase::Reviewing => "reviewing",
            TaskPhase::Merging => "merging",
        }
    }
}

// ============================================================================
// Idle Reason
// ============================================================================

/// Reason for being in Idle state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IdleReason {
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

impl IdleReason {
    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            IdleReason::Done => "done",
            IdleReason::HumanReview => "human_review",
            IdleReason::Error => "error",
            IdleReason::Conflict => "conflict",
            IdleReason::Timeout => "timeout",
            IdleReason::Manual => "manual",
        }
    }
}

// ============================================================================
// Task State
// ============================================================================

/// Runtime state for a single task (v2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskState {
    /// Current status (process state)
    pub status: TaskStatus,

    /// Current phase (business progress)
    #[serde(default, skip_serializing_if = "is_phase_none")]
    pub phase: TaskPhase,

    /// Reason for being idle (only when status is Idle)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idle_reason: Option<IdleReason>,

    /// Timestamp when entered Active state (for monitoring)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_since: Option<DateTime<Utc>>,

    /// Instance information (worktree, branch, session)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance: Option<Instance>,

    /// Scratch environment flag (v1 compat)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scratch: Option<bool>,
}

fn is_phase_none(phase: &TaskPhase) -> bool {
    matches!(phase, TaskPhase::None)
}

impl Default for TaskState {
    fn default() -> Self {
        Self {
            status: TaskStatus::Pending,
            phase: TaskPhase::None,
            idle_reason: None,
            active_since: None,
            instance: None,
            scratch: None,
        }
    }
}

impl TaskState {
    /// Transition to Active state (test only - production code sets fields directly)
    #[cfg(test)]
    pub fn to_active(&mut self, phase: TaskPhase) {
        self.status = TaskStatus::Active;
        self.phase = phase;
        self.idle_reason = None;
        self.active_since = Some(Utc::now());
    }

    /// Transition to Idle state
    pub fn to_idle(&mut self, reason: IdleReason) {
        self.status = TaskStatus::Idle;
        self.idle_reason = Some(reason);
        self.active_since = None;
    }

    /// Convert TaskPhase to phase ID string
    ///
    /// Returns None for TaskPhase::None (pending state).
    pub fn phase_id(&self) -> Option<&'static str> {
        match self.phase {
            TaskPhase::None => None,
            TaskPhase::Developing => Some("developing"),
            TaskPhase::Reviewing => Some("reviewing"),
            TaskPhase::Merging => Some("merging"),
        }
    }
}

// ============================================================================
// Status Store
// ============================================================================

/// Store for all task runtime states (v2)
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StatusStore {
    pub tasks: HashMap<String, TaskState>,
}

impl StatusStore {
    /// Load status from .wt/status.json
    pub fn load() -> Result<Self> {
        let path = Path::new(STATUS_FILE);
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path).map_err(|e| WtError::Io {
            operation: "read status file".to_string(),
            path: STATUS_FILE.to_string(),
            message: e.to_string(),
        })?;

        serde_json::from_str(&content)
            .map_err(|e| WtError::InvalidTaskFile(format!("Invalid status.json: {}", e)))
    }

    /// Save status to .wt/status.json (atomic write)
    pub fn save(&self) -> Result<()> {
        let path = Path::new(STATUS_FILE);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| WtError::Io {
                    operation: "create status directory".to_string(),
                    path: parent.to_string_lossy().to_string(),
                    message: e.to_string(),
                })?;
            }
        }

        let content = serde_json::to_string_pretty(&self)
            .map_err(|e| WtError::InvalidTaskFile(format!("Failed to serialize status: {}", e)))?;

        // Atomic write: temp file + rename
        let temp_path = format!("{}.tmp", STATUS_FILE);
        fs::write(&temp_path, &content).map_err(|e| WtError::Io {
            operation: "write temp status file".to_string(),
            path: temp_path.clone(),
            message: e.to_string(),
        })?;

        fs::rename(&temp_path, path).map_err(|e| WtError::Io {
            operation: "rename status file".to_string(),
            path: STATUS_FILE.to_string(),
            message: e.to_string(),
        })?;

        Ok(())
    }

    /// Get state for a task (default: Pending)
    pub fn get(&self, name: &str) -> TaskState {
        self.tasks.get(name).cloned().unwrap_or_default()
    }

    /// Get mutable reference to task state, creating default if not exists
    pub fn get_mut(&mut self, name: &str) -> &mut TaskState {
        self.tasks.entry(name.to_string()).or_default()
    }

    /// Get status for a task
    pub fn get_status(&self, name: &str) -> TaskStatus {
        self.tasks
            .get(name)
            .map(|s| s.status.clone())
            .unwrap_or_default()
    }

    /// Get instance for a task
    pub fn get_instance(&self, name: &str) -> Option<&Instance> {
        self.tasks.get(name).and_then(|s| s.instance.as_ref())
    }

    /// Set instance for a task
    pub fn set_instance(&mut self, name: &str, instance: Option<Instance>) {
        self.get_mut(name).instance = instance;
    }

    /// Set status for a task (v1 compat)
    pub fn set_status(&mut self, name: &str, status: TaskStatus) {
        self.get_mut(name).status = status;
    }

    /// Get phase for a task
    pub fn get_phase(&self, name: &str) -> Option<&TaskPhase> {
        self.tasks.get(name).map(|s| &s.phase)
    }

    /// Get idle reason for a task
    pub fn get_idle_reason(&self, name: &str) -> Option<&IdleReason> {
        self.tasks.get(name).and_then(|s| s.idle_reason.as_ref())
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

    // ==================== TaskPhase Tests ====================

    #[test]
    fn test_phase_default() {
        let phase: TaskPhase = Default::default();
        assert_eq!(phase, TaskPhase::None);
    }

    #[test]
    fn test_phase_serialize() {
        assert_eq!(
            serde_json::to_string(&TaskPhase::None).unwrap(),
            "\"none\""
        );
        assert_eq!(
            serde_json::to_string(&TaskPhase::Developing).unwrap(),
            "\"developing\""
        );
        assert_eq!(
            serde_json::to_string(&TaskPhase::Reviewing).unwrap(),
            "\"reviewing\""
        );
        assert_eq!(
            serde_json::to_string(&TaskPhase::Merging).unwrap(),
            "\"merging\""
        );
    }

    // ==================== IdleReason Tests ====================

    #[test]
    fn test_idle_reason_serialize() {
        assert_eq!(
            serde_json::to_string(&IdleReason::Done).unwrap(),
            "\"done\""
        );
        assert_eq!(
            serde_json::to_string(&IdleReason::HumanReview).unwrap(),
            "\"human_review\""
        );
        assert_eq!(
            serde_json::to_string(&IdleReason::Error).unwrap(),
            "\"error\""
        );
        assert_eq!(
            serde_json::to_string(&IdleReason::Conflict).unwrap(),
            "\"conflict\""
        );
        assert_eq!(
            serde_json::to_string(&IdleReason::Timeout).unwrap(),
            "\"timeout\""
        );
        assert_eq!(
            serde_json::to_string(&IdleReason::Manual).unwrap(),
            "\"manual\""
        );
    }

    // ==================== TaskState Tests ====================

    #[test]
    fn test_state_default() {
        let state = TaskState::default();
        assert_eq!(state.status, TaskStatus::Pending);
        assert_eq!(state.phase, TaskPhase::None);
        assert!(state.idle_reason.is_none());
        assert!(state.active_since.is_none());
        assert!(state.instance.is_none());
    }

    #[test]
    fn test_state_transitions() {
        let mut state = TaskState::default();

        // Pending → Active
        state.to_active(TaskPhase::Developing);
        assert_eq!(state.status, TaskStatus::Active);
        assert_eq!(state.phase, TaskPhase::Developing);
        assert!(state.active_since.is_some());

        // Active → Idle
        state.to_idle(IdleReason::Done);
        assert_eq!(state.status, TaskStatus::Idle);
        assert_eq!(state.idle_reason, Some(IdleReason::Done));
        assert!(state.active_since.is_none());

        // Idle → Active
        state.to_active(TaskPhase::Reviewing);
        assert_eq!(state.status, TaskStatus::Active);
        assert_eq!(state.phase, TaskPhase::Reviewing);
    }

    #[test]
    fn test_state_serialize_minimal() {
        let state = TaskState::default();
        let json = serde_json::to_string(&state).unwrap();

        // Should only have status, phase=none should be skipped
        assert!(json.contains("\"status\":\"pending\""));
        assert!(!json.contains("phase"));
        assert!(!json.contains("idle_reason"));
        assert!(!json.contains("active_since"));
    }

    #[test]
    fn test_state_serialize_full() {
        let mut state = TaskState::default();
        state.to_active(TaskPhase::Developing);
        state.instance = Some(Instance {
            branch: "wt/test".to_string(),
            worktree_path: "/path".to_string(),
            session_name: "wt".to_string(),
            window_name: "test".to_string(),
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
        assert_eq!(state.phase, TaskPhase::None);
    }

    // ==================== StatusStore Tests ====================

    #[test]
    fn test_store_default() {
        let store = StatusStore::default();
        assert!(store.tasks.is_empty());
    }

    #[test]
    fn test_store_get_default() {
        let store = StatusStore::default();
        let state = store.get("nonexistent");
        assert_eq!(state.status, TaskStatus::Pending);
    }

    #[test]
    fn test_store_get_mut() {
        let mut store = StatusStore::default();
        {
            let state = store.get_mut("test");
            state.to_active(TaskPhase::Developing);
        }

        let got = store.get("test");
        assert_eq!(got.status, TaskStatus::Active);
    }

    #[test]
    fn test_store_serialize() {
        let mut store = StatusStore::default();
        store.get_mut("task1").to_active(TaskPhase::Developing);
        store.get_mut("task2").to_idle(IdleReason::Done);

        let json = serde_json::to_string(&store).unwrap();
        assert!(json.contains("task1"));
        assert!(json.contains("task2"));
        assert!(json.contains("active"));
        assert!(json.contains("idle"));
    }

    #[test]
    fn test_store_deserialize() {
        let json = r#"{
            "tasks": {
                "test": {
                    "status": "active",
                    "phase": "developing",
                    "active_since": "2026-02-03T10:30:00Z"
                }
            }
        }"#;
        let store: StatusStore = serde_json::from_str(json).unwrap();

        let state = store.get("test");
        assert_eq!(state.status, TaskStatus::Active);
        assert_eq!(state.phase, TaskPhase::Developing);
        assert!(state.active_since.is_some());
    }

    // ==================== Phases v2 Bridge Tests ====================

    #[test]
    fn test_task_state_phase_id() {
        let mut state = TaskState::default();
        assert_eq!(state.phase_id(), None);

        state.phase = TaskPhase::Developing;
        assert_eq!(state.phase_id(), Some("developing"));

        state.phase = TaskPhase::Reviewing;
        assert_eq!(state.phase_id(), Some("reviewing"));

        state.phase = TaskPhase::Merging;
        assert_eq!(state.phase_id(), Some("merging"));
    }
}
