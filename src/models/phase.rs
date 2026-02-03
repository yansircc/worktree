//! Phase model for Phases v2 system.
//!
//! Phase represents a stage in the task lifecycle with:
//! - on_enter: workflow to run when entering the phase
//! - on_exit: workflow to run when leaving the phase
//! - resources: whether worktree/branch are needed
//! - prerequisites: conditions to enter this phase

use serde::{Deserialize, Serialize};

use super::workflow::{Workflow, WorkflowState};

// ============================================================================
// PhaseState
// ============================================================================

/// Phase execution state (derived from workflow state)
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PhaseState {
    /// Not started
    #[default]
    Pending,
    /// Workflow is executing
    Running,
    /// Workflow completed successfully
    Success,
    /// Workflow failed
    Failed,
    /// Workflow blocked (needs intervention)
    Blocked,
}

impl PhaseState {
    /// Derive phase state from workflow state
    pub fn derive(workflow_state: &WorkflowState) -> Self {
        match workflow_state {
            WorkflowState::Pending => PhaseState::Pending,
            WorkflowState::Running => PhaseState::Running,
            WorkflowState::Success => PhaseState::Success,
            WorkflowState::Failed => PhaseState::Failed,
            WorkflowState::Blocked => PhaseState::Blocked,
        }
    }

    /// Check if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, PhaseState::Success | PhaseState::Failed | PhaseState::Blocked)
    }

    /// Check if phase completed successfully
    pub fn is_success(&self) -> bool {
        matches!(self, PhaseState::Success)
    }

    /// Get display icon
    pub fn icon(&self) -> &'static str {
        match self {
            PhaseState::Pending => "○",
            PhaseState::Running => "●",
            PhaseState::Success => "✓",
            PhaseState::Failed => "✗",
            PhaseState::Blocked => "⊘",
        }
    }
}

// ============================================================================
// Resource Requirements
// ============================================================================

/// Resource requirements for a phase
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PhaseResources {
    /// No resources needed (e.g., pending, completed)
    #[default]
    None,
    /// Full resources: worktree, branch, multiplexer window
    Full,
}

impl PhaseResources {
    /// Check if worktree is required
    pub fn needs_worktree(&self) -> bool {
        matches!(self, PhaseResources::Full)
    }

    /// Check if branch is required
    pub fn needs_branch(&self) -> bool {
        matches!(self, PhaseResources::Full)
    }

    /// Check if multiplexer window is required
    pub fn needs_window(&self) -> bool {
        matches!(self, PhaseResources::Full)
    }
}

// ============================================================================
// Prerequisites
// ============================================================================

/// Dependency completion requirement
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DependencyRequirement {
    /// All dependencies must be completed
    #[default]
    Completed,
    /// Any state is acceptable
    Any,
}

/// Prerequisites to enter a phase
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhasePrerequisites {
    /// Dependency completion requirement
    #[serde(default)]
    pub dependencies: DependencyRequirement,
    /// Allowed source phases (empty = any)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phase: Vec<String>,
    /// Additional condition (variable expression)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
}

// ============================================================================
// Exit Reason
// ============================================================================

/// Reason for exiting a phase
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExitReason {
    /// Workflow completed successfully
    #[default]
    Success,
    /// User forced transition via `wt next`
    Forced,
    /// Workflow failed, then user forced transition
    Failed,
}

// ============================================================================
// Timeout Config
// ============================================================================

/// Timeout action
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TimeoutAction {
    /// Mark phase as blocked
    #[default]
    Block,
    /// Mark phase as failed
    Fail,
    /// Just notify, don't change state
    Notify,
}

/// Phase timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseTimeout {
    /// Timeout duration (e.g., "4h", "1d")
    pub duration: String,
    /// Action to take on timeout
    #[serde(default)]
    pub action: TimeoutAction,
}

// ============================================================================
// Notification Config
// ============================================================================

/// Notification backend
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NotificationBackend {
    /// No notification
    None,
    /// Slack notification
    Slack,
    /// Email notification
    Email,
    /// System notification
    System,
}

impl Default for NotificationBackend {
    fn default() -> Self {
        NotificationBackend::None
    }
}

/// Phase notification configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhaseNotifications {
    /// Notification on blocked
    #[serde(default)]
    pub on_blocked: NotificationBackend,
    /// Notification on success
    #[serde(default)]
    pub on_success: NotificationBackend,
    /// Notification on failure
    #[serde(default)]
    pub on_failure: NotificationBackend,
}

// ============================================================================
// Observe Config
// ============================================================================

/// Phase observation configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhaseObserve {
    /// Show progress indicator
    #[serde(default)]
    pub progress: bool,
    /// Notification configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notifications: Option<PhaseNotifications>,
}

// ============================================================================
// Phase
// ============================================================================

/// A phase in the task lifecycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase {
    /// Phase identifier (e.g., "developing", "reviewing")
    pub id: String,

    /// Human-readable name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Description/goal of this phase
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal: Option<String>,

    /// Resource requirements
    #[serde(default)]
    pub resources: PhaseResources,

    /// Prerequisites to enter this phase
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prerequisites: Option<PhasePrerequisites>,

    /// Workflow to run on entering this phase
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_enter: Option<Workflow>,

    /// Workflow to run on exiting this phase
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_exit: Option<Workflow>,

    /// Observation configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observe: Option<PhaseObserve>,

    /// Timeout configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<PhaseTimeout>,
}

impl Phase {
    /// Create a new phase with just an ID
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: None,
            goal: None,
            resources: PhaseResources::None,
            prerequisites: None,
            on_enter: None,
            on_exit: None,
            observe: None,
            timeout: None,
        }
    }

    /// Create a phase that requires full resources
    pub fn with_resources(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: None,
            goal: None,
            resources: PhaseResources::Full,
            prerequisites: None,
            on_enter: None,
            on_exit: None,
            observe: None,
            timeout: None,
        }
    }

    /// Get phase display name
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }

    /// Check if resources are required
    pub fn needs_resources(&self) -> bool {
        self.resources.needs_worktree()
    }

    /// Set on_enter workflow
    pub fn with_on_enter(mut self, workflow: Workflow) -> Self {
        self.on_enter = Some(workflow);
        self
    }

    /// Set on_exit workflow
    pub fn with_on_exit(mut self, workflow: Workflow) -> Self {
        self.on_exit = Some(workflow);
        self
    }
}

// ============================================================================
// Default Phases
// ============================================================================

/// Standard phase sequence for most projects
pub const DEFAULT_PHASE_SEQUENCE: &[&str] = &["pending", "developing", "reviewing", "completed"];

/// Create default phase definitions
pub fn default_phases() -> Vec<Phase> {
    vec![
        Phase::new("pending"),
        Phase::with_resources("developing"),
        Phase::with_resources("reviewing"),
        Phase::new("completed"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_state_derive() {
        assert_eq!(PhaseState::derive(&WorkflowState::Pending), PhaseState::Pending);
        assert_eq!(PhaseState::derive(&WorkflowState::Running), PhaseState::Running);
        assert_eq!(PhaseState::derive(&WorkflowState::Success), PhaseState::Success);
        assert_eq!(PhaseState::derive(&WorkflowState::Failed), PhaseState::Failed);
        assert_eq!(PhaseState::derive(&WorkflowState::Blocked), PhaseState::Blocked);
    }

    #[test]
    fn test_phase_resources() {
        let none = PhaseResources::None;
        assert!(!none.needs_worktree());
        assert!(!none.needs_branch());
        assert!(!none.needs_window());

        let full = PhaseResources::Full;
        assert!(full.needs_worktree());
        assert!(full.needs_branch());
        assert!(full.needs_window());
    }

    #[test]
    fn test_phase_creation() {
        let phase = Phase::new("developing");
        assert_eq!(phase.id, "developing");
        assert_eq!(phase.resources, PhaseResources::None);

        let phase = Phase::with_resources("developing");
        assert_eq!(phase.id, "developing");
        assert_eq!(phase.resources, PhaseResources::Full);
    }

    #[test]
    fn test_default_phases() {
        let phases = default_phases();
        assert_eq!(phases.len(), 4);
        assert_eq!(phases[0].id, "pending");
        assert_eq!(phases[0].resources, PhaseResources::None);
        assert_eq!(phases[1].id, "developing");
        assert_eq!(phases[1].resources, PhaseResources::Full);
    }

    #[test]
    fn test_phase_serialize() {
        let phase = Phase::with_resources("developing");
        let json = serde_json::to_string(&phase).unwrap();
        assert!(json.contains("developing"));
        assert!(json.contains("full"));

        let parsed: Phase = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "developing");
        assert_eq!(parsed.resources, PhaseResources::Full);
    }
}
