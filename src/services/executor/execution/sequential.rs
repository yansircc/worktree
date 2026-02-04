//! Sequential execution strategy.
//!
//! Executes steps one after another, passing context between them.

use crate::models::step::{OnError, Step, StepResult, StepState};
use crate::models::workflow::{OnStepBlocked, OnStepFailure, Workflow};
use crate::services::observer::sync::SyncObservers;

/// Execute steps sequentially.
///
/// Steps run one after another. Each step can access context from previous steps.
/// Execution stops early if a step fails and abort is configured.
pub fn execute_sequential<F>(
    workflow: &Workflow,
    on_failure: &OnStepFailure,
    on_blocked: &OnStepBlocked,
    _observers: &SyncObservers,
    mut execute_step: F,
) -> Vec<StepResult>
where
    F: FnMut(&Step, usize, Option<&StepResult>) -> StepResult,
{
    let mut results: Vec<StepResult> = Vec::new();

    for (index, step) in workflow.steps.iter().enumerate() {
        // Pass previous result for context
        let prev = results.last();
        let result = execute_step(step, index, prev);

        // Store step output for later steps
        let should_abort = should_abort(step, &result, on_failure, on_blocked);
        results.push(result);

        if should_abort {
            break;
        }
    }

    results
}

/// Check if execution should abort based on step result and on_error config.
///
/// Priority: step.on_error > workflow.on_step_failure
fn should_abort(
    step: &Step,
    result: &StepResult,
    on_failure: &OnStepFailure,
    on_blocked: &OnStepBlocked,
) -> bool {
    match result.state {
        StepState::Failed | StepState::Timeout => {
            // Check step-level on_error first
            match step.effective_on_error() {
                OnError::Inherit => matches!(on_failure, OnStepFailure::Abort),
                OnError::Fail => true,
                OnError::Continue => false,
                OnError::Block => true, // Block also stops execution
                OnError::Retry => false, // Retry handled by execute_step_with_retry
            }
        }
        StepState::Blocked => {
            matches!(on_blocked, OnStepBlocked::Abort | OnStepBlocked::Pause)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::step::OnError;

    #[test]
    fn test_should_abort_on_failure_with_abort() {
        let step = Step::script("test");
        let result = StepResult {
            state: StepState::Failed,
            ..Default::default()
        };

        assert!(should_abort(
            &step,
            &result,
            &OnStepFailure::Abort,
            &OnStepBlocked::Pause
        ));
    }

    #[test]
    fn test_should_not_abort_on_failure_with_continue() {
        let step = Step::script("test");
        let result = StepResult {
            state: StepState::Failed,
            ..Default::default()
        };

        assert!(!should_abort(
            &step,
            &result,
            &OnStepFailure::Continue,
            &OnStepBlocked::Pause
        ));
    }

    #[test]
    fn test_step_on_error_overrides_workflow() {
        let mut step = Step::script("test");
        step.on_error = Some(OnError::Continue);
        let result = StepResult {
            state: StepState::Failed,
            ..Default::default()
        };

        // Step says continue, workflow says abort - step wins
        assert!(!should_abort(
            &step,
            &result,
            &OnStepFailure::Abort,
            &OnStepBlocked::Pause
        ));
    }
}
