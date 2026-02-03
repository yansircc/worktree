//! State derivation for Phases v2 system.
//!
//! Implements the state derivation chain:
//! ```text
//! StepState (each step)
//!     │ aggregate
//!     ▼
//! WorkflowState (current workflow)
//!     │ map
//!     ▼
//! PhaseState (current phase)
//!     │ + resources
//!     ▼
//! TaskStatus (task state)
//!     │ aggregate
//!     ▼
//! ProjectStatus (project state)
//! ```

use serde::{Deserialize, Serialize};

use super::phase::PhaseState;
use super::project::ProjectStatus;
use super::step::StepResult;
#[cfg(test)]
use super::step::StepState;
use super::workflow::WorkflowState;

// ============================================================================
// Task Status (v2)
// ============================================================================

/// Task status (derived from phase state + resources)
///
/// Note: This replaces the old TaskStatus in status.rs
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DerivedTaskStatus {
    /// Task defined but not started (no phase, no resources)
    #[default]
    Pending,
    /// A process is running (phase running/success, has resources)
    Active,
    /// Resources exist but no process running (phase blocked/failed)
    Idle,
    /// Task completed (phase = completed, no resources)
    Completed,
}

impl DerivedTaskStatus {
    /// Derive task status from phase ID and phase state
    ///
    /// Rules:
    /// - phase == null → Pending
    /// - phase == "completed" → Completed
    /// - phase running/success → Active (about to auto-advance)
    /// - phase blocked/failed → Idle
    pub fn derive(phase_id: Option<&str>, phase_state: &PhaseState) -> Self {
        match phase_id {
            None => DerivedTaskStatus::Pending,
            Some("completed") => DerivedTaskStatus::Completed,
            Some(_) => match phase_state {
                PhaseState::Running => DerivedTaskStatus::Active,
                PhaseState::Success => DerivedTaskStatus::Active, // Will auto-advance
                PhaseState::Blocked | PhaseState::Failed => DerivedTaskStatus::Idle,
                PhaseState::Pending => DerivedTaskStatus::Idle,
            },
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            DerivedTaskStatus::Pending => "pending",
            DerivedTaskStatus::Active => "active",
            DerivedTaskStatus::Idle => "idle",
            DerivedTaskStatus::Completed => "completed",
        }
    }

    /// Get status icon
    pub fn icon(&self) -> &'static str {
        match self {
            DerivedTaskStatus::Pending => "○",
            DerivedTaskStatus::Active => "●",
            DerivedTaskStatus::Idle => "◐",
            DerivedTaskStatus::Completed => "✓",
        }
    }

    /// Check if task can be started
    pub fn can_start(&self) -> bool {
        matches!(self, DerivedTaskStatus::Pending | DerivedTaskStatus::Idle)
    }

    /// Check if task is finished
    pub fn is_finished(&self) -> bool {
        matches!(self, DerivedTaskStatus::Completed)
    }
}

// ============================================================================
// State Derivation Functions
// ============================================================================

/// Derive workflow state from step results
pub fn derive_workflow_state(step_results: &[StepResult]) -> WorkflowState {
    WorkflowState::derive(step_results)
}

/// Derive phase state from workflow state
pub fn derive_phase_state(workflow_state: &WorkflowState) -> PhaseState {
    PhaseState::derive(workflow_state)
}

/// Derive task status from phase info
pub fn derive_task_status(phase_id: Option<&str>, phase_state: &PhaseState) -> DerivedTaskStatus {
    DerivedTaskStatus::derive(phase_id, phase_state)
}

/// Derive project status from task statuses
pub fn derive_project_status(task_statuses: &[DerivedTaskStatus]) -> ProjectStatus {
    let pending = task_statuses
        .iter()
        .filter(|s| **s == DerivedTaskStatus::Pending)
        .count();
    let active = task_statuses
        .iter()
        .filter(|s| **s == DerivedTaskStatus::Active)
        .count();
    let idle = task_statuses
        .iter()
        .filter(|s| **s == DerivedTaskStatus::Idle)
        .count();
    let completed = task_statuses
        .iter()
        .filter(|s| **s == DerivedTaskStatus::Completed)
        .count();

    ProjectStatus::new(pending, active, idle, completed)
}

// ============================================================================
// Full Derivation Chain
// ============================================================================

/// Complete derivation from step results to task status
pub fn derive_task_status_from_steps(
    step_results: &[StepResult],
    phase_id: Option<&str>,
) -> (WorkflowState, PhaseState, DerivedTaskStatus) {
    let workflow_state = derive_workflow_state(step_results);
    let phase_state = derive_phase_state(&workflow_state);
    let task_status = derive_task_status(phase_id, &phase_state);
    (workflow_state, phase_state, task_status)
}

// ============================================================================
// Runtime State
// ============================================================================

/// Runtime state for a task
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskRuntimeState {
    /// Current phase ID (None = pending, "completed" = done)
    pub phase_id: Option<String>,
    /// Current workflow state
    pub workflow_state: WorkflowState,
    /// Current step index in workflow
    pub current_step: usize,
    /// Results of completed steps
    pub step_results: Vec<StepResult>,
    /// Whether this is running on_enter or on_exit
    #[serde(default)]
    pub is_on_exit: bool,
}

impl TaskRuntimeState {
    /// Create state for a pending task
    pub fn pending() -> Self {
        Self::default()
    }

    /// Create state for a completed task
    pub fn completed() -> Self {
        Self {
            phase_id: Some("completed".to_string()),
            workflow_state: WorkflowState::Success,
            ..Default::default()
        }
    }

    /// Get derived task status
    pub fn task_status(&self) -> DerivedTaskStatus {
        let phase_state = derive_phase_state(&self.workflow_state);
        derive_task_status(self.phase_id.as_deref(), &phase_state)
    }

    /// Get phase state
    pub fn phase_state(&self) -> PhaseState {
        derive_phase_state(&self.workflow_state)
    }

    /// Update from step results
    pub fn update_from_steps(&mut self) {
        self.workflow_state = derive_workflow_state(&self.step_results);
    }

    /// Transition to a new phase
    pub fn transition_to(&mut self, phase_id: impl Into<String>) {
        self.phase_id = Some(phase_id.into());
        self.workflow_state = WorkflowState::Pending;
        self.current_step = 0;
        self.step_results.clear();
        self.is_on_exit = false;
    }

    /// Start on_exit workflow
    pub fn start_on_exit(&mut self) {
        self.workflow_state = WorkflowState::Pending;
        self.current_step = 0;
        self.step_results.clear();
        self.is_on_exit = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derived_task_status_pending() {
        let status = DerivedTaskStatus::derive(None, &PhaseState::Pending);
        assert_eq!(status, DerivedTaskStatus::Pending);
    }

    #[test]
    fn test_derived_task_status_completed() {
        let status = DerivedTaskStatus::derive(Some("completed"), &PhaseState::Success);
        assert_eq!(status, DerivedTaskStatus::Completed);
    }

    #[test]
    fn test_derived_task_status_active() {
        let status = DerivedTaskStatus::derive(Some("developing"), &PhaseState::Running);
        assert_eq!(status, DerivedTaskStatus::Active);

        let status = DerivedTaskStatus::derive(Some("developing"), &PhaseState::Success);
        assert_eq!(status, DerivedTaskStatus::Active);
    }

    #[test]
    fn test_derived_task_status_idle() {
        let status = DerivedTaskStatus::derive(Some("developing"), &PhaseState::Blocked);
        assert_eq!(status, DerivedTaskStatus::Idle);

        let status = DerivedTaskStatus::derive(Some("developing"), &PhaseState::Failed);
        assert_eq!(status, DerivedTaskStatus::Idle);
    }

    #[test]
    fn test_full_derivation_chain() {
        // Simulate: step running
        let step_results = vec![StepResult {
            state: StepState::Running,
            ..Default::default()
        }];

        let (workflow, phase, task) = derive_task_status_from_steps(&step_results, Some("developing"));
        assert_eq!(workflow, WorkflowState::Running);
        assert_eq!(phase, PhaseState::Running);
        assert_eq!(task, DerivedTaskStatus::Active);
    }

    #[test]
    fn test_full_derivation_blocked() {
        let step_results = vec![
            StepResult {
                state: StepState::Success,
                ..Default::default()
            },
            StepResult {
                state: StepState::Blocked,
                ..Default::default()
            },
        ];

        let (workflow, phase, task) = derive_task_status_from_steps(&step_results, Some("developing"));
        assert_eq!(workflow, WorkflowState::Blocked);
        assert_eq!(phase, PhaseState::Blocked);
        assert_eq!(task, DerivedTaskStatus::Idle);
    }

    #[test]
    fn test_project_status_derivation() {
        let statuses = vec![
            DerivedTaskStatus::Pending,
            DerivedTaskStatus::Active,
            DerivedTaskStatus::Idle,
            DerivedTaskStatus::Completed,
            DerivedTaskStatus::Completed,
        ];

        let project = derive_project_status(&statuses);
        assert_eq!(project.total, 5);
        assert_eq!(project.pending, 1);
        assert_eq!(project.active, 1);
        assert_eq!(project.idle, 1);
        assert_eq!(project.completed, 2);
        assert!((project.progress - 0.4).abs() < 0.01);
    }

    #[test]
    fn test_task_runtime_state() {
        let mut state = TaskRuntimeState::pending();
        assert_eq!(state.task_status(), DerivedTaskStatus::Pending);

        state.transition_to("developing");
        state.step_results.push(StepResult {
            state: StepState::Running,
            ..Default::default()
        });
        state.update_from_steps();
        assert_eq!(state.task_status(), DerivedTaskStatus::Active);

        let completed = TaskRuntimeState::completed();
        assert_eq!(completed.task_status(), DerivedTaskStatus::Completed);
    }
}
