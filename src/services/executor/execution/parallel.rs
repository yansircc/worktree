//! Parallel execution strategy.
//!
//! Executes all steps concurrently using rayon thread pool.

use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;

use crate::models::step::{OnError, Step, StepResult, StepState};
use crate::models::workflow::{OnStepBlocked, OnStepFailure, Workflow};
use crate::services::observer::sync::SyncObservers;

/// Execute steps in parallel using rayon thread pool.
///
/// All steps run concurrently with optional thread limit via `max_parallel`.
/// Results are returned in original step order regardless of completion order.
pub fn execute_parallel<F>(
    workflow: &Workflow,
    on_failure: &OnStepFailure,
    on_blocked: &OnStepBlocked,
    max_parallel: Option<usize>,
    _observers: &SyncObservers,
    execute_step: F,
) -> Vec<StepResult>
where
    F: Fn(&Step, usize) -> StepResult + Sync,
{
    // Early abort flag for Abort mode
    let aborted = AtomicBool::new(false);

    // Build thread pool with optional parallelism limit
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(max_parallel.unwrap_or(0)) // 0 = use all available
        .build()
        .unwrap_or_else(|_| rayon::ThreadPoolBuilder::new().build().unwrap());

    pool.install(|| {
        workflow
            .steps
            .par_iter()
            .enumerate()
            .map(|(index, step)| {
                // Check if already aborted
                if aborted.load(Ordering::Relaxed) {
                    return StepResult {
                        step_id: step.id.clone(),
                        state: StepState::Skipped,
                        message: Some("Aborted due to earlier failure".to_string()),
                        ..Default::default()
                    };
                }

                let result = execute_step(step, index);

                // Signal abort if needed
                if should_abort(step, &result, on_failure, on_blocked) {
                    aborted.store(true, Ordering::Relaxed);
                }

                result
            })
            .collect()
    })
}

/// Check if execution should abort based on step result and on_error config.
fn should_abort(
    step: &Step,
    result: &StepResult,
    on_failure: &OnStepFailure,
    on_blocked: &OnStepBlocked,
) -> bool {
    match result.state {
        StepState::Failed | StepState::Timeout => {
            match step.effective_on_error() {
                OnError::Inherit => matches!(on_failure, OnStepFailure::Abort),
                OnError::Fail => true,
                OnError::Continue => false,
                OnError::Block => true,
                OnError::Retry => false,
            }
        }
        StepState::Blocked => {
            matches!(on_blocked, OnStepBlocked::Abort | OnStepBlocked::Pause)
        }
        _ => false,
    }
}
