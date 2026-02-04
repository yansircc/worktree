//! DAG-based execution strategy.
//!
//! Executes steps in batches based on dependency order.
//! Steps within each batch run in parallel.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;

use crate::models::step::{OnError, Step, StepResult, StepState};
use crate::models::workflow::{OnStepBlocked, OnStepFailure, Workflow};
use crate::services::observer::sync::SyncObservers;

/// Execute steps based on DAG with parallel batch execution.
///
/// Steps are grouped into batches by dependency order. Within each batch,
/// steps run in parallel. Batches execute sequentially (later batches wait
/// for earlier ones to complete).
pub fn execute_dag<F>(
    workflow: &Workflow,
    on_failure: &OnStepFailure,
    on_blocked: &OnStepBlocked,
    max_parallel: Option<usize>,
    observers: &SyncObservers,
    execute_step: F,
) -> Vec<StepResult>
where
    F: Fn(&Step, usize) -> StepResult + Sync,
{
    let mut results: Vec<Option<StepResult>> = vec![None; workflow.steps.len()];
    let execution_order = workflow.execution_order();

    // Build thread pool with optional parallelism limit
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(max_parallel.unwrap_or(0))
        .build()
        .unwrap_or_else(|_| rayon::ThreadPoolBuilder::new().build().unwrap());

    for batch in execution_order {
        // Check if any dependency failed (for skip_dependents mode)
        let skip_indices: HashSet<usize> = if matches!(on_failure, OnStepFailure::SkipDependents) {
            batch
                .iter()
                .filter(|&&idx| {
                    workflow.steps[idx].depends.iter().any(|dep_id| {
                        workflow.steps.iter().enumerate().any(|(i, s)| {
                            s.id.as_deref() == Some(dep_id.as_str())
                                && results[i]
                                    .as_ref()
                                    .map(|r| r.state == StepState::Failed)
                                    .unwrap_or(false)
                        })
                    })
                })
                .copied()
                .collect()
        } else {
            HashSet::new()
        };

        // Early abort flag
        let aborted = AtomicBool::new(false);

        // Execute batch in parallel
        let batch_results: Vec<(usize, StepResult)> = pool.install(|| {
            batch
                .par_iter()
                .map(|&idx| {
                    let step = &workflow.steps[idx];
                    let step_name = step.id.clone().unwrap_or_else(|| format!("step-{}", idx));

                    // Check skip
                    if skip_indices.contains(&idx) {
                        let result = StepResult {
                            step_id: step.id.clone(),
                            state: StepState::Skipped,
                            message: Some("Dependency failed".to_string()),
                            ..Default::default()
                        };
                        observers.on_step_complete(idx, &step_name, &result);
                        return (idx, result);
                    }

                    // Check abort
                    if aborted.load(Ordering::Relaxed) {
                        let result = StepResult {
                            step_id: step.id.clone(),
                            state: StepState::Skipped,
                            message: Some("Aborted due to earlier failure".to_string()),
                            ..Default::default()
                        };
                        return (idx, result);
                    }

                    let result = execute_step(step, idx);

                    // Signal abort if needed
                    if should_abort(step, &result, on_failure, on_blocked) {
                        aborted.store(true, Ordering::Relaxed);
                    }

                    (idx, result)
                })
                .collect()
        });

        // Store batch results
        let mut should_abort_workflow = false;
        for (idx, result) in &batch_results {
            let step = &workflow.steps[*idx];
            if should_abort(step, result, on_failure, on_blocked) {
                should_abort_workflow = true;
            }
        }
        for (idx, result) in batch_results {
            results[idx] = Some(result);
        }

        // Abort remaining batches if needed
        if should_abort_workflow {
            for r in results.iter_mut() {
                if r.is_none() {
                    *r = Some(StepResult::default());
                }
            }
            return results.into_iter().flatten().collect();
        }
    }

    results.into_iter().flatten().collect()
}

/// Check if execution should abort based on step result and on_error config.
fn should_abort(
    step: &Step,
    result: &StepResult,
    on_failure: &OnStepFailure,
    on_blocked: &OnStepBlocked,
) -> bool {
    match result.state {
        StepState::Failed | StepState::Timeout => match step.effective_on_error() {
            OnError::Inherit => matches!(on_failure, OnStepFailure::Abort),
            OnError::Fail => true,
            OnError::Continue => false,
            OnError::Block => true,
            OnError::Retry => false,
        },
        StepState::Blocked => {
            matches!(on_blocked, OnStepBlocked::Abort | OnStepBlocked::Pause)
        }
        _ => false,
    }
}
