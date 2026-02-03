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
    /// Checkpoint step index for resume (last successfully completed step)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint_step: Option<usize>,
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
        self.checkpoint_step = None;
    }

    /// Start on_exit workflow
    pub fn start_on_exit(&mut self) {
        self.workflow_state = WorkflowState::Pending;
        self.current_step = 0;
        self.step_results.clear();
        self.is_on_exit = true;
        self.checkpoint_step = None;
    }

    /// Update checkpoint after a successful step
    pub fn update_checkpoint(&mut self, step_index: usize) {
        self.checkpoint_step = Some(step_index);
        self.current_step = step_index + 1;
    }

    /// Get the step index to resume from (0 if no checkpoint)
    pub fn resume_from(&self) -> usize {
        self.checkpoint_step.map(|i| i + 1).unwrap_or(0)
    }

    /// Check if we can resume from a checkpoint
    pub fn can_resume(&self) -> bool {
        self.checkpoint_step.is_some() && !self.step_results.is_empty()
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

    #[test]
    fn test_checkpoint_update() {
        let mut state = TaskRuntimeState::pending();
        state.transition_to("developing");

        state.update_checkpoint(0);
        assert_eq!(state.checkpoint_step, Some(0));
        assert_eq!(state.current_step, 1);
        assert_eq!(state.resume_from(), 1);

        state.update_checkpoint(2);
        assert_eq!(state.checkpoint_step, Some(2));
        assert_eq!(state.resume_from(), 3);
    }

    #[test]
    fn test_can_resume() {
        let mut state = TaskRuntimeState::pending();
        assert!(!state.can_resume());

        state.checkpoint_step = Some(0);
        assert!(!state.can_resume()); // No step_results yet

        state.step_results.push(StepResult::default());
        assert!(state.can_resume());
    }

    #[test]
    fn test_checkpoint_cleared_on_transition() {
        let mut state = TaskRuntimeState::pending();
        state.checkpoint_step = Some(2);
        state.transition_to("reviewing");
        assert!(state.checkpoint_step.is_none());
    }
}
