//! Status model for Agent Hooks system.
//!
//! Implements the state model from `.claude/specs/agent-hooks.md`:
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
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
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
    /// Check if transition to target status is valid
    pub fn can_transition_to(&self, target: &TaskStatus) -> bool {
        matches!(
            (self, target),
            // Pending → Active (wt run)
            (TaskStatus::Pending, TaskStatus::Active)
            // Active → Idle (process ends)
            | (TaskStatus::Active, TaskStatus::Idle)
            // Idle → Active (wt resume, wt review, etc.)
            | (TaskStatus::Idle, TaskStatus::Active)
            // Active → Completed (wt complete succeeds)
            | (TaskStatus::Active, TaskStatus::Completed)
            // Idle → Completed (direct completion without running process)
            | (TaskStatus::Idle, TaskStatus::Completed)
            // Any → Pending (wt reset)
            | (_, TaskStatus::Pending)
        )
    }

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
    /// Create a new Pending state
    #[allow(dead_code)] // Constructor API for tests and future use
    pub fn pending() -> Self {
        Self::default()
    }

    /// Create an Active state with the given phase
    #[allow(dead_code)] // Constructor API for tests and future use
    pub fn active(phase: TaskPhase) -> Self {
        Self {
            status: TaskStatus::Active,
            phase,
            idle_reason: None,
            active_since: Some(Utc::now()),
            instance: None,
            scratch: None,
        }
    }

    /// Create an Idle state with the given phase and reason
    #[allow(dead_code)] // Constructor API for tests and future use
    pub fn idle(phase: TaskPhase, reason: IdleReason) -> Self {
        Self {
            status: TaskStatus::Idle,
            phase,
            idle_reason: Some(reason),
            active_since: None,
            instance: None,
            scratch: None,
        }
    }

    /// Create a Completed state
    #[allow(dead_code)] // Constructor API for tests and future use
    pub fn completed() -> Self {
        Self {
            status: TaskStatus::Completed,
            phase: TaskPhase::None,
            idle_reason: None,
            active_since: None,
            instance: None,
            scratch: None,
        }
    }

    /// Transition to Active state
    #[allow(dead_code)] // State machine API for future use
    pub fn to_active(&mut self, phase: TaskPhase) {
        self.status = TaskStatus::Active;
        self.phase = phase;
        self.idle_reason = None;
        self.active_since = Some(Utc::now());
    }

    /// Transition to Idle state
    #[allow(dead_code)] // State machine API for future use
    pub fn to_idle(&mut self, reason: IdleReason) {
        self.status = TaskStatus::Idle;
        self.idle_reason = Some(reason);
        self.active_since = None;
    }

    /// Transition to Completed state
    #[allow(dead_code)] // State machine API for future use
    pub fn to_completed(&mut self) {
        self.status = TaskStatus::Completed;
        self.phase = TaskPhase::None;
        self.idle_reason = None;
        self.active_since = None;
        self.instance = None;
    }

    /// Reset to Pending state
    #[allow(dead_code)] // State machine API for future use
    pub fn to_pending(&mut self) {
        self.status = TaskStatus::Pending;
        self.phase = TaskPhase::None;
        self.idle_reason = None;
        self.active_since = None;
        self.instance = None;
    }

    /// Check if the task is in an error state
    #[allow(dead_code)] // Query API for future use
    pub fn is_error(&self) -> bool {
        self.status == TaskStatus::Idle && self.idle_reason == Some(IdleReason::Error)
    }

    /// Check if the task has a conflict
    #[allow(dead_code)] // Query API for future use
    pub fn has_conflict(&self) -> bool {
        self.status == TaskStatus::Idle && self.idle_reason == Some(IdleReason::Conflict)
    }

    /// Get duration since entering Active state
    #[allow(dead_code)] // Query API for future use
    pub fn active_duration(&self) -> Option<chrono::Duration> {
        self.active_since.map(|since| Utc::now() - since)
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

    /// Set state for a task
    #[allow(dead_code)] // API for tests and future use
    pub fn set(&mut self, name: &str, state: TaskState) {
        self.tasks.insert(name.to_string(), state);
    }

    /// Remove a task (used by delete command)
    #[allow(dead_code)] // API for tests and future use
    pub fn remove(&mut self, name: &str) -> Option<TaskState> {
        self.tasks.remove(name)
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
    fn test_status_transitions() {
        // Valid transitions
        assert!(TaskStatus::Pending.can_transition_to(&TaskStatus::Active));
        assert!(TaskStatus::Active.can_transition_to(&TaskStatus::Idle));
        assert!(TaskStatus::Idle.can_transition_to(&TaskStatus::Active));
        assert!(TaskStatus::Active.can_transition_to(&TaskStatus::Completed));
        assert!(TaskStatus::Idle.can_transition_to(&TaskStatus::Completed));

        // Reset transitions (any → Pending)
        assert!(TaskStatus::Active.can_transition_to(&TaskStatus::Pending));
        assert!(TaskStatus::Idle.can_transition_to(&TaskStatus::Pending));
        assert!(TaskStatus::Completed.can_transition_to(&TaskStatus::Pending));

        // Invalid transitions
        assert!(!TaskStatus::Pending.can_transition_to(&TaskStatus::Idle));
        assert!(!TaskStatus::Pending.can_transition_to(&TaskStatus::Completed));
        assert!(!TaskStatus::Completed.can_transition_to(&TaskStatus::Active));
        assert!(!TaskStatus::Completed.can_transition_to(&TaskStatus::Idle));
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
    fn test_state_constructors() {
        let pending = TaskState::pending();
        assert_eq!(pending.status, TaskStatus::Pending);

        let active = TaskState::active(TaskPhase::Developing);
        assert_eq!(active.status, TaskStatus::Active);
        assert_eq!(active.phase, TaskPhase::Developing);
        assert!(active.active_since.is_some());

        let idle = TaskState::idle(TaskPhase::Reviewing, IdleReason::Done);
        assert_eq!(idle.status, TaskStatus::Idle);
        assert_eq!(idle.phase, TaskPhase::Reviewing);
        assert_eq!(idle.idle_reason, Some(IdleReason::Done));

        let completed = TaskState::completed();
        assert_eq!(completed.status, TaskStatus::Completed);
    }

    #[test]
    fn test_state_transitions() {
        let mut state = TaskState::pending();

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

        // Active → Completed
        state.to_completed();
        assert_eq!(state.status, TaskStatus::Completed);
        assert_eq!(state.phase, TaskPhase::None);
    }

    #[test]
    fn test_state_reset() {
        let mut state = TaskState::active(TaskPhase::Developing);
        state.to_pending();

        assert_eq!(state.status, TaskStatus::Pending);
        assert_eq!(state.phase, TaskPhase::None);
        assert!(state.idle_reason.is_none());
        assert!(state.active_since.is_none());
    }

    #[test]
    fn test_state_is_error() {
        let mut state = TaskState::idle(TaskPhase::Developing, IdleReason::Error);
        assert!(state.is_error());

        state.idle_reason = Some(IdleReason::Done);
        assert!(!state.is_error());

        state.status = TaskStatus::Active;
        state.idle_reason = Some(IdleReason::Error);
        assert!(!state.is_error()); // Not Idle status
    }

    #[test]
    fn test_state_has_conflict() {
        let state = TaskState::idle(TaskPhase::Merging, IdleReason::Conflict);
        assert!(state.has_conflict());

        let state = TaskState::idle(TaskPhase::Merging, IdleReason::Error);
        assert!(!state.has_conflict());
    }

    #[test]
    fn test_state_serialize_minimal() {
        let state = TaskState::pending();
        let json = serde_json::to_string(&state).unwrap();

        // Should only have status, phase=none should be skipped
        assert!(json.contains("\"status\":\"pending\""));
        assert!(!json.contains("phase"));
        assert!(!json.contains("idle_reason"));
        assert!(!json.contains("active_since"));
    }

    #[test]
    fn test_state_serialize_full() {
        let mut state = TaskState::active(TaskPhase::Developing);
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
    fn test_store_set_and_get() {
        let mut store = StatusStore::default();
        let state = TaskState::active(TaskPhase::Developing);
        store.set("test", state);

        let got = store.get("test");
        assert_eq!(got.status, TaskStatus::Active);
        assert_eq!(got.phase, TaskPhase::Developing);
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
    fn test_store_remove() {
        let mut store = StatusStore::default();
        store.set("test", TaskState::active(TaskPhase::Developing));

        let removed = store.remove("test");
        assert!(removed.is_some());
        assert!(store.tasks.get("test").is_none());

        // Get after remove should return default
        assert_eq!(store.get("test").status, TaskStatus::Pending);
    }

    #[test]
    fn test_store_serialize() {
        let mut store = StatusStore::default();
        store.set("task1", TaskState::active(TaskPhase::Developing));
        store.set("task2", TaskState::idle(TaskPhase::Reviewing, IdleReason::Done));

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
}
