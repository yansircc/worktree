//! Runtime state for task execution.

use serde::{Deserialize, Serialize};

use super::step::StepResult;
use super::workflow::WorkflowState;

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
    fn test_task_runtime_state_pending() {
        let state = TaskRuntimeState::pending();
        assert!(state.phase_id.is_none());
        assert_eq!(state.workflow_state, WorkflowState::Pending);
    }

    #[test]
    fn test_task_runtime_state_transition() {
        let mut state = TaskRuntimeState::pending();
        state.transition_to("developing");
        assert_eq!(state.phase_id, Some("developing".to_string()));
        assert_eq!(state.workflow_state, WorkflowState::Pending);
        assert!(!state.is_on_exit);
    }

    #[test]
    fn test_task_runtime_state_on_exit() {
        let mut state = TaskRuntimeState::pending();
        state.transition_to("developing");
        state.start_on_exit();
        assert!(state.is_on_exit);
        assert_eq!(state.workflow_state, WorkflowState::Pending);
    }
}
