//! Phase transition for Phases v2.
//!
//! Manages phase lifecycle:
//! - Resource allocation/deallocation
//! - on_enter workflow execution
//! - on_exit workflow execution
//! - State transitions

use std::path::PathBuf;

use crate::error::{Result, WtError};
use crate::models::phase::{ExitReason, Phase, PhaseState};
use crate::models::state::TaskRuntimeState;
use crate::models::workflow::WorkflowState;
use crate::models::WtConfig;
use crate::services::executor::context::ExecutionContext;
use crate::services::executor::workflow::{WorkflowExecutor, WorkflowResult};

/// Result of phase transition
#[derive(Debug)]
#[allow(dead_code)] // Fields are populated but callers may not read all of them
pub struct PhaseTransitionResult {
    /// New phase ID
    pub phase_id: String,
    /// Phase state after transition
    pub phase_state: PhaseState,
    /// on_enter workflow result (if any)
    pub on_enter_result: Option<WorkflowResult>,
}

/// Phase transition manager
pub struct PhaseTransition<'a> {
    config: &'a WtConfig,
    context: ExecutionContext,
    log_dir: Option<PathBuf>,
    show_progress: bool,
    /// Total steps in original workflow (for partial execution display)
    total_steps: Option<usize>,
}

impl<'a> PhaseTransition<'a> {
    /// Create a new phase transition manager.
    pub fn new(config: &'a WtConfig, context: ExecutionContext) -> Self {
        Self {
            config,
            context,
            log_dir: None,
            show_progress: false,
            total_steps: None,
        }
    }

    /// Set log directory.
    pub fn with_log_dir(mut self, dir: PathBuf) -> Self {
        self.log_dir = Some(dir);
        self
    }

    /// Enable terminal progress output.
    pub fn with_progress(mut self, show: bool) -> Self {
        self.show_progress = show;
        self
    }

    /// Set total steps for partial execution display.
    pub fn with_total_steps(mut self, total: usize) -> Self {
        self.total_steps = Some(total);
        self
    }

    /// Enter a new phase.
    ///
    /// Steps:
    /// 1. Validate prerequisites (if any)
    /// 2. Allocate resources (if needed)
    /// 3. Execute on_enter workflow (if any)
    /// 4. Return new phase state
    pub fn enter(
        &self,
        phase: &Phase,
        runtime_state: &mut TaskRuntimeState,
    ) -> Result<PhaseTransitionResult> {
        // Validate prerequisites
        if let Some(ref prereqs) = phase.prerequisites {
            // Check allowed source phases
            if !prereqs.phase.is_empty() {
                // If no current phase (None), we're in pending state
                match runtime_state.phase_id.as_deref() {
                    Some(current) if prereqs.phase.iter().any(|p| p == current) => {
                        // Current phase is in the allowed list
                    }
                    None if prereqs.phase.iter().any(|p| p == "none") => {
                        // No current phase and "none" is in the allowed list
                    }
                    _ => {
                        let current_name = runtime_state.phase_id.as_deref().unwrap_or("(none)");
                        return Err(WtError::InvalidInput(format!(
                            "Cannot transition to {} from {}",
                            phase.id, current_name
                        )));
                    }
                }
            }

            // Check condition
            if let Some(ref condition) = prereqs.condition {
                let expanded = self.context.expand(condition);
                if !self.evaluate_condition(&expanded) {
                    return Err(WtError::InvalidInput(format!(
                        "Prerequisite condition not met: {}",
                        condition
                    )));
                }
            }
        }

        // Update runtime state
        runtime_state.transition_to(&phase.id);

        // Update context for this phase
        let phase_context = self.context.clone().with_phase(&phase.id);

        // Execute on_enter workflow
        let on_enter_result = if let Some(ref workflow) = phase.on_enter {
            let mut executor = WorkflowExecutor::new(self.config, phase_context.clone())
                .with_log_dir(self.get_phase_log_dir(&phase.id))
                .with_progress(self.show_progress);

            if let Some(total) = self.total_steps {
                executor = executor.with_total_steps(total);
            }

            let result = executor.execute(workflow)?;

            // Update runtime state from workflow result
            runtime_state.step_results = result.step_results.clone();
            runtime_state.workflow_state = result.state.clone();

            Some(result)
        } else {
            // No on_enter workflow - mark as success
            runtime_state.workflow_state = WorkflowState::Success;
            None
        };

        let phase_state = PhaseState::derive(&runtime_state.workflow_state);

        Ok(PhaseTransitionResult {
            phase_id: phase.id.clone(),
            phase_state,
            on_enter_result,
        })
    }

    /// Exit current phase.
    ///
    /// Steps:
    /// 1. Execute on_exit workflow (if any)
    /// 2. Deallocate resources (if needed for next phase)
    /// 3. Return exit result
    pub fn exit(
        &self,
        phase: &Phase,
        exit_reason: ExitReason,
        runtime_state: &mut TaskRuntimeState,
    ) -> Result<Option<WorkflowResult>> {
        // Mark as exiting
        runtime_state.start_on_exit();

        // Update context with exit reason
        let exit_context = self.context.clone()
            .with_phase(&phase.id)
            .with_exit_reason(exit_reason);

        // Execute on_exit workflow
        if let Some(ref workflow) = phase.on_exit {
            let executor = WorkflowExecutor::new(self.config, exit_context)
                .with_log_dir(self.get_phase_log_dir(&phase.id))
                .with_progress(self.show_progress);

            let result = executor.execute(workflow)?;

            // Update runtime state
            runtime_state.step_results = result.step_results.clone();
            runtime_state.workflow_state = result.state.clone();

            Ok(Some(result))
        } else {
            runtime_state.workflow_state = WorkflowState::Success;
            Ok(None)
        }
    }

    /// Get log directory for a phase.
    fn get_phase_log_dir(&self, phase_id: &str) -> PathBuf {
        self.log_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from(".wt/logs"))
            .join(&self.context.task)
            .join(phase_id)
    }

    /// Evaluate a condition.
    fn evaluate_condition(&self, condition: &str) -> bool {
        // Simple expression evaluation
        if condition.contains("==") {
            let parts: Vec<&str> = condition.split("==").map(|s| s.trim()).collect();
            if parts.len() == 2 {
                return parts[0].trim_matches('\'').trim_matches('"')
                    == parts[1].trim_matches('\'').trim_matches('"');
            }
        }
        if condition.contains("!=") {
            let parts: Vec<&str> = condition.split("!=").map(|s| s.trim()).collect();
            if parts.len() == 2 {
                return parts[0].trim_matches('\'').trim_matches('"')
                    != parts[1].trim_matches('\'').trim_matches('"');
            }
        }

        // Fall back to shell
        std::process::Command::new("sh")
            .arg("-c")
            .arg(condition)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

/// Determine next phase in sequence.
pub fn next_phase<'a>(
    current: Option<&str>,
    sequence: &'a [String],
) -> Option<&'a str> {
    match current {
        None => sequence.first().map(|s| s.as_str()),
        Some(id) => {
            let idx = sequence.iter().position(|s| s == id)?;
            sequence.get(idx + 1).map(|s| s.as_str())
        }
    }
}

/// Determine previous phase in sequence.
pub fn prev_phase<'a>(
    current: Option<&str>,
    sequence: &'a [String],
) -> Option<&'a str> {
    match current {
        None => None,
        Some(id) => {
            let idx = sequence.iter().position(|s| s == id)?;
            if idx == 0 {
                None
            } else {
                sequence.get(idx - 1).map(|s| s.as_str())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::phase::{Phase, PhasePrerequisites};
    use crate::models::workflow::Workflow;

    fn test_config() -> WtConfig {
        WtConfig::default()
    }

    fn test_context() -> ExecutionContext {
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string());
        ExecutionContext::new("auth", "wt/auth", &cwd, &cwd)
    }

    #[test]
    fn test_enter_simple_phase() {
        let config = test_config();
        let context = test_context();
        let transition = PhaseTransition::new(&config, context);

        let phase = Phase::new("developing");
        let mut runtime = TaskRuntimeState::pending();

        let result = transition.enter(&phase, &mut runtime).unwrap();

        assert_eq!(result.phase_id, "developing");
        assert_eq!(result.phase_state, PhaseState::Success);
        assert!(result.on_enter_result.is_none());
    }

    #[test]
    fn test_enter_phase_with_on_enter() {
        let config = test_config();
        let context = test_context();
        let transition = PhaseTransition::new(&config, context);

        let phase = Phase::new("developing")
            .with_on_enter(Workflow::from_scripts(&["true"]));
        let mut runtime = TaskRuntimeState::pending();

        let result = transition.enter(&phase, &mut runtime).unwrap();

        assert!(result.on_enter_result.is_some());
        assert_eq!(result.phase_state, PhaseState::Success);
    }

    #[test]
    fn test_enter_phase_on_enter_fails() {
        let config = test_config();
        let context = test_context();
        let transition = PhaseTransition::new(&config, context);

        let phase = Phase::new("developing")
            .with_on_enter(Workflow::from_scripts(&["exit 1"]));
        let mut runtime = TaskRuntimeState::pending();

        let result = transition.enter(&phase, &mut runtime).unwrap();

        assert_eq!(result.phase_state, PhaseState::Failed);
    }

    #[test]
    fn test_exit_phase() {
        let config = test_config();
        let context = test_context();
        let transition = PhaseTransition::new(&config, context);

        let phase = Phase::new("developing")
            .with_on_exit(Workflow::from_scripts(&["true"]));
        let mut runtime = TaskRuntimeState::pending();
        runtime.transition_to("developing");

        let result = transition.exit(&phase, ExitReason::Success, &mut runtime).unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().state, WorkflowState::Success);
    }

    #[test]
    fn test_next_phase() {
        let sequence: Vec<String> = vec![
            "pending".to_string(),
            "developing".to_string(),
            "reviewing".to_string(),
            "completed".to_string(),
        ];

        assert_eq!(next_phase(None, &sequence), Some("pending"));
        assert_eq!(next_phase(Some("pending"), &sequence), Some("developing"));
        assert_eq!(next_phase(Some("developing"), &sequence), Some("reviewing"));
        assert_eq!(next_phase(Some("completed"), &sequence), None);
        assert_eq!(next_phase(Some("unknown"), &sequence), None);
    }

    #[test]
    fn test_prev_phase() {
        let sequence: Vec<String> = vec![
            "pending".to_string(),
            "developing".to_string(),
            "reviewing".to_string(),
            "completed".to_string(),
        ];

        assert_eq!(prev_phase(None, &sequence), None);
        assert_eq!(prev_phase(Some("pending"), &sequence), None);
        assert_eq!(prev_phase(Some("developing"), &sequence), Some("pending"));
        assert_eq!(prev_phase(Some("completed"), &sequence), Some("reviewing"));
    }

    #[test]
    fn test_enter_phase_prerequisite_wrong_phase() {
        let config = test_config();
        let context = test_context();
        let transition = PhaseTransition::new(&config, context);

        // Phase that requires coming from "pending"
        let phase = Phase::new("developing")
            .with_prerequisites(PhasePrerequisites {
                phase: vec!["pending".to_string()],
                ..Default::default()
            });

        // But runtime is at "reviewing" (not "pending")
        let mut runtime = TaskRuntimeState::pending();
        runtime.transition_to("reviewing");

        let result = transition.enter(&phase, &mut runtime);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Cannot transition"));
    }

    #[test]
    fn test_enter_phase_prerequisite_condition_fails() {
        let config = test_config();
        let context = test_context();
        let transition = PhaseTransition::new(&config, context);

        // Phase with a failing condition
        let phase = Phase::new("developing")
            .with_prerequisites(PhasePrerequisites {
                condition: Some("false".to_string()),
                ..Default::default()
            });
        let mut runtime = TaskRuntimeState::pending();

        let result = transition.enter(&phase, &mut runtime);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("condition not met"));
    }

    #[test]
    fn test_exit_phase_on_exit_fails() {
        let config = test_config();
        let context = test_context();
        let transition = PhaseTransition::new(&config, context);

        let phase = Phase::new("developing")
            .with_on_exit(Workflow::from_scripts(&["exit 1"]));
        let mut runtime = TaskRuntimeState::pending();
        runtime.transition_to("developing");

        let result = transition.exit(&phase, ExitReason::Success, &mut runtime).unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().state, WorkflowState::Failed);
    }

    #[test]
    fn test_exit_phase_no_workflow() {
        let config = test_config();
        let context = test_context();
        let transition = PhaseTransition::new(&config, context);

        let phase = Phase::new("developing");
        let mut runtime = TaskRuntimeState::pending();
        runtime.transition_to("developing");

        let result = transition.exit(&phase, ExitReason::Forced, &mut runtime).unwrap();

        assert!(result.is_none());
        assert_eq!(runtime.workflow_state, WorkflowState::Success);
    }

    #[test]
    fn test_evaluate_condition_equality() {
        let config = test_config();
        let context = test_context();
        let transition = PhaseTransition::new(&config, context);

        assert!(transition.evaluate_condition("'a' == 'a'"));
        assert!(!transition.evaluate_condition("'a' == 'b'"));
        assert!(transition.evaluate_condition("'a' != 'b'"));
        assert!(!transition.evaluate_condition("'a' != 'a'"));
    }

    #[test]
    fn test_evaluate_condition_shell() {
        let config = test_config();
        let context = test_context();
        let transition = PhaseTransition::new(&config, context);

        assert!(transition.evaluate_condition("true"));
        assert!(!transition.evaluate_condition("false"));
        assert!(transition.evaluate_condition("test 1 -eq 1"));
    }
}
