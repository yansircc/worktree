//! Workflow executor for Phases v2.
//!
//! Orchestrates step execution with support for:
//! - Sequential execution
//! - Parallel execution (using rayon thread pool)
//! - DAG-based execution (batches run in parallel)
//! - Observer integration (terminal progress, file logging)

use std::path::PathBuf;

use chrono::Utc;

use crate::error::Result;
use crate::models::step::{OnError, Step, StepResult, StepState};
use crate::models::workflow::{ExecutionMode, OnStepBlocked, OnStepFailure, Workflow, WorkflowState};
use crate::models::WtConfig;
use crate::services::executor::context::ExecutionContext;
use crate::services::executor::execution::{execute_dag, execute_parallel, execute_sequential};
use crate::services::executor::step::StepExecutor;
use crate::services::observer::log::{create_workflow_log_entry, LogObserver};
use crate::services::observer::sync::SyncObservers;
use crate::services::observer::terminal::{TerminalObserver, TerminalSettings};

/// Result of workflow execution
#[derive(Debug)]
pub struct WorkflowResult {
    /// Final workflow state
    pub state: WorkflowState,
    /// Results from each step
    pub step_results: Vec<StepResult>,
    /// Total duration in milliseconds
    #[allow(dead_code)] // Populated but not always read
    pub duration_ms: u64,
}

/// Workflow executor
pub struct WorkflowExecutor<'a> {
    config: &'a WtConfig,
    context: ExecutionContext,
    log_dir: Option<PathBuf>,
    /// Show terminal progress output
    show_progress: bool,
}

impl<'a> WorkflowExecutor<'a> {
    /// Create a new workflow executor.
    pub fn new(config: &'a WtConfig, context: ExecutionContext) -> Self {
        Self {
            config,
            context,
            log_dir: None,
            show_progress: false,
        }
    }

    /// Set log directory for step outputs.
    pub fn with_log_dir(mut self, dir: PathBuf) -> Self {
        self.log_dir = Some(dir);
        self
    }

    /// Enable terminal progress output.
    pub fn with_progress(mut self, show: bool) -> Self {
        self.show_progress = show;
        self
    }

    /// Execute a workflow.
    pub fn execute(&self, workflow: &Workflow) -> Result<WorkflowResult> {
        let start = std::time::Instant::now();
        let started_at = Utc::now();

        // Create observers
        let terminal_observer = if self.show_progress {
            Some(TerminalObserver::new(TerminalSettings {
                show_progress: true,
                ..Default::default()
            }))
        } else {
            None
        };

        let log_observer = if let Some(ref dir) = self.log_dir {
            let mut obs = LogObserver::new(dir).with_stream(true);
            let _ = obs.init();
            Some(obs)
        } else {
            None
        };

        if workflow.is_empty() {
            return Ok(WorkflowResult {
                state: WorkflowState::Success,
                step_results: Vec::new(),
                duration_ms: 0,
            });
        }

        let workflow_name = workflow.id.clone().unwrap_or_else(|| "workflow".to_string());

        // Notify workflow start
        if let Some(ref obs) = terminal_observer {
            obs.on_workflow_start(&workflow_name, workflow.steps.len());
        }

        let execution_config = workflow.execution.as_ref();
        let mode = execution_config
            .map(|e| e.mode.clone())
            .unwrap_or_default();
        let on_failure = execution_config
            .map(|e| e.on_step_failure.clone())
            .unwrap_or_default();
        let on_blocked = execution_config
            .map(|e| e.on_step_blocked.clone())
            .unwrap_or_default();
        let max_parallel = execution_config
            .and_then(|e| e.max_parallel);

        // Create thread-safe observers for parallel execution
        let sync_observers = SyncObservers::new(terminal_observer, log_observer);

        let step_results = match mode {
            ExecutionMode::Sequential => {
                self.run_sequential(workflow, &on_failure, &on_blocked, &sync_observers)
            }
            ExecutionMode::Parallel => {
                self.run_parallel(workflow, &on_failure, &on_blocked, max_parallel, &sync_observers)
            }
            ExecutionMode::Dag => {
                self.run_dag(workflow, &on_failure, &on_blocked, max_parallel, &sync_observers)
            }
        };

        let state = WorkflowState::derive(&step_results);
        let duration_ms = start.elapsed().as_millis() as u64;

        // Extract terminal observer for final notification
        let log_observer = sync_observers.into_log_observer();

        // Notify workflow complete (terminal observer was moved into sync_observers)
        if self.show_progress {
            let obs = TerminalObserver::new(TerminalSettings {
                show_progress: true,
                ..Default::default()
            });
            obs.on_workflow_complete(&workflow_name, &state, duration_ms);
        }

        // Save workflow context/summary to log
        if let Some(ref obs) = log_observer {
            let entry = create_workflow_log_entry(
                workflow.id.as_deref(),
                &workflow_name,
                state.clone(),
                &step_results,
                duration_ms,
                started_at,
            );
            let _ = obs.save_workflow_context(&entry);
        }

        Ok(WorkflowResult {
            state,
            step_results,
            duration_ms,
        })
    }


    // =========================================================================
    // Execution mode dispatchers
    // =========================================================================

    /// Run sequential execution mode.
    fn run_sequential(
        &self,
        workflow: &Workflow,
        on_failure: &OnStepFailure,
        on_blocked: &OnStepBlocked,
        observers: &SyncObservers,
    ) -> Vec<StepResult> {
        let mut context = self.context.clone();

        execute_sequential(
            workflow,
            on_failure,
            on_blocked,
            observers,
            |step, index, prev| {
                // Update context for this step
                context = context.clone().next_step(index, step.id.as_deref());

                // Add previous step info
                if let Some(prev_result) = prev {
                    context.prev_state = Some(format!("{:?}", prev_result.state).to_lowercase());
                }

                // Store step output for later steps
                if let Some(prev_result) = prev {
                    if let Some(ref id) = prev_result.step_id {
                        if prev_result.state == StepState::Success {
                            context.step_outputs.insert(id.clone(), String::new());
                        }
                    }
                }

                self.execute_step_with_retry(step, index, context.clone(), observers)
            },
        )
    }

    /// Run parallel execution mode.
    fn run_parallel(
        &self,
        workflow: &Workflow,
        on_failure: &OnStepFailure,
        on_blocked: &OnStepBlocked,
        max_parallel: Option<usize>,
        observers: &SyncObservers,
    ) -> Vec<StepResult> {
        execute_parallel(
            workflow,
            on_failure,
            on_blocked,
            max_parallel,
            observers,
            |step, index| {
                let context = self.context.clone().next_step(index, step.id.as_deref());
                self.execute_step_with_retry(step, index, context, observers)
            },
        )
    }

    /// Run DAG execution mode.
    fn run_dag(
        &self,
        workflow: &Workflow,
        on_failure: &OnStepFailure,
        on_blocked: &OnStepBlocked,
        max_parallel: Option<usize>,
        observers: &SyncObservers,
    ) -> Vec<StepResult> {
        execute_dag(
            workflow,
            on_failure,
            on_blocked,
            max_parallel,
            observers,
            |step, index| {
                let context = self.context.clone().next_step(index, step.id.as_deref());
                self.execute_step_with_retry(step, index, context, observers)
            },
        )
    }

    // =========================================================================
    // Step execution helpers
    // =========================================================================

    /// Execute a single step with thread-safe observer notifications.
    ///
    /// This method is safe to call from multiple threads concurrently.
    fn execute_step_with_sync_observers(
        &self,
        step: &Step,
        index: usize,
        context: ExecutionContext,
        observers: &SyncObservers,
    ) -> StepResult {
        let step_name = step.id.clone().unwrap_or_else(|| format!("step-{}", index));

        // Notify step start (thread-safe)
        observers.on_step_start(index, step.id.as_deref(), &step_name);

        // Execute step
        let mut executor = StepExecutor::new(self.config, context);
        if let Some(ref dir) = self.log_dir {
            executor = executor.with_log_dir(dir.clone());
        }
        let result = executor.execute(step);

        // Notify step complete (thread-safe)
        observers.on_step_complete(index, &step_name, &result);

        result
    }

    /// Execute a single step with retry support.
    ///
    /// If step has on_error: retry and retry config, this will retry on failure.
    fn execute_step_with_retry(
        &self,
        step: &Step,
        index: usize,
        context: ExecutionContext,
        observers: &SyncObservers,
    ) -> StepResult {
        use crate::models::step::parse_duration;

        // Check if retry is enabled for this step
        let should_retry = matches!(step.effective_on_error(), OnError::Retry);
        if !should_retry {
            return self.execute_step_with_sync_observers(step, index, context, observers);
        }

        let retry_config = step.retry.as_ref();
        let max_attempts = retry_config.map(|r| r.max_attempts).unwrap_or(2);
        let delay = retry_config
            .and_then(|r| r.delay.as_deref())
            .and_then(parse_duration);

        let step_name = step.id.clone().unwrap_or_else(|| format!("step-{}", index));

        for attempt in 0..max_attempts {
            // Execute the step
            let mut result = self.execute_step_with_sync_observers(
                step, index, context.clone(), observers
            );
            result.attempt = attempt;

            // Check if retry is needed
            let should_retry_now = matches!(result.state, StepState::Failed | StepState::Timeout)
                && attempt + 1 < max_attempts;

            if !should_retry_now {
                return result;
            }

            // Notify retry
            let delay_ms = delay.map(|d| d.as_millis() as u64).unwrap_or(0);
            observers.on_step_retry(index, &step_name, attempt, max_attempts, delay_ms);

            // Wait before retry
            if let Some(d) = delay {
                std::thread::sleep(d);
            }
        }

        // Should not reach here, but return a failed result just in case
        StepResult {
            step_id: step.id.clone(),
            state: StepState::Failed,
            message: Some("Max retries exceeded".to_string()),
            attempt: max_attempts - 1,
            ..Default::default()
        }
    }

}

// Test-only methods
#[cfg(test)]
impl<'a> WorkflowExecutor<'a> {
    /// Resume workflow execution from previously saved results (test only).
    pub fn resume(
        &self,
        workflow: &Workflow,
        saved_results: Vec<StepResult>,
    ) -> Result<WorkflowResult> {
        let start = std::time::Instant::now();

        // Find the resume point: first step that didn't succeed
        let resume_from = saved_results
            .iter()
            .position(|r| !matches!(r.state, StepState::Success | StepState::Skipped))
            .unwrap_or(saved_results.len());

        // If all steps completed successfully, return the saved results
        if resume_from >= workflow.steps.len() {
            let state = WorkflowState::derive(&saved_results);
            return Ok(WorkflowResult {
                state,
                step_results: saved_results,
                duration_ms: 0,
            });
        }

        let sync_observers = SyncObservers::new(None, None);

        // Copy preserved results
        let mut results: Vec<StepResult> = saved_results[..resume_from].to_vec();

        // Get execution config
        let execution_config = workflow.execution.as_ref();
        let on_failure = execution_config
            .map(|e| e.on_step_failure.clone())
            .unwrap_or_default();
        let on_blocked = execution_config
            .map(|e| e.on_step_blocked.clone())
            .unwrap_or_default();

        // Execute remaining steps sequentially
        let mut context = self.context.clone();

        for (index, step) in workflow.steps.iter().enumerate().skip(resume_from) {
            context = context.next_step(index, step.id.as_deref());

            if let Some(prev) = results.last() {
                context.prev_state = Some(format!("{:?}", prev.state).to_lowercase());
            }

            let result =
                self.execute_step_with_retry(step, index, context.clone(), &sync_observers);

            let should_abort = Self::should_abort(step, &result, &on_failure, &on_blocked);
            results.push(result);

            if should_abort {
                break;
            }
        }

        let state = WorkflowState::derive(&results);
        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(WorkflowResult {
            state,
            step_results: results,
            duration_ms,
        })
    }

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::step::Step;
    use crate::models::workflow::ExecutionConfig;

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
    fn test_execute_empty_workflow() {
        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        let workflow = Workflow::default();
        let result = executor.execute(&workflow).unwrap();

        assert_eq!(result.state, WorkflowState::Success);
        assert!(result.step_results.is_empty());
    }

    #[test]
    fn test_execute_sequential_success() {
        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        let workflow = Workflow::from_scripts(&["true", "true"]);
        let result = executor.execute(&workflow).unwrap();

        assert_eq!(result.state, WorkflowState::Success);
        assert_eq!(result.step_results.len(), 2);
        assert!(result.step_results.iter().all(|r| r.state == StepState::Success));
    }

    #[test]
    fn test_execute_sequential_failure_abort() {
        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        let workflow = Workflow {
            steps: vec![
                Step::script("true"),
                Step::script("exit 1"),
                Step::script("true"), // Should not run
            ],
            execution: Some(ExecutionConfig {
                on_step_failure: OnStepFailure::Abort,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = executor.execute(&workflow).unwrap();

        assert_eq!(result.state, WorkflowState::Failed);
        assert_eq!(result.step_results.len(), 2); // Third step not executed
        assert_eq!(result.step_results[0].state, StepState::Success);
        assert_eq!(result.step_results[1].state, StepState::Failed);
    }

    #[test]
    fn test_execute_sequential_failure_continue() {
        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        let workflow = Workflow {
            steps: vec![
                Step::script("true"),
                Step::script("exit 1"),
                Step::script("true"),
            ],
            execution: Some(ExecutionConfig {
                on_step_failure: OnStepFailure::Continue,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = executor.execute(&workflow).unwrap();

        assert_eq!(result.state, WorkflowState::Failed);
        assert_eq!(result.step_results.len(), 3);
        assert_eq!(result.step_results[2].state, StepState::Success);
    }

    #[test]
    fn test_execute_dag() {
        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        let workflow = Workflow {
            steps: vec![
                Step {
                    id: Some("install".to_string()),
                    run: Some("true".to_string()),
                    depends: vec![],
                    ..Step::script("")
                },
                Step {
                    id: Some("lint".to_string()),
                    run: Some("true".to_string()),
                    depends: vec!["install".to_string()],
                    ..Step::script("")
                },
                Step {
                    id: Some("test".to_string()),
                    run: Some("true".to_string()),
                    depends: vec!["install".to_string()],
                    ..Step::script("")
                },
                Step {
                    id: Some("build".to_string()),
                    run: Some("true".to_string()),
                    depends: vec!["lint".to_string(), "test".to_string()],
                    ..Step::script("")
                },
            ],
            execution: Some(ExecutionConfig {
                mode: ExecutionMode::Dag,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = executor.execute(&workflow).unwrap();

        assert_eq!(result.state, WorkflowState::Success);
        assert_eq!(result.step_results.len(), 4);
        assert!(result.step_results.iter().all(|r| r.state == StepState::Success));
    }

    #[test]
    fn test_execute_dag_skip_dependents() {
        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        let workflow = Workflow {
            steps: vec![
                Step {
                    id: Some("install".to_string()),
                    run: Some("exit 1".to_string()), // Fails
                    depends: vec![],
                    ..Step::script("")
                },
                Step {
                    id: Some("build".to_string()),
                    run: Some("true".to_string()),
                    depends: vec!["install".to_string()], // Should be skipped
                    ..Step::script("")
                },
            ],
            execution: Some(ExecutionConfig {
                mode: ExecutionMode::Dag,
                on_step_failure: OnStepFailure::SkipDependents,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = executor.execute(&workflow).unwrap();

        assert_eq!(result.state, WorkflowState::Failed);
        assert_eq!(result.step_results[0].state, StepState::Failed);
        assert_eq!(result.step_results[1].state, StepState::Skipped);
    }

    #[test]
    fn test_workflow_result_duration() {
        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        let workflow = Workflow::from_scripts(&["sleep 0.1"]);
        let result = executor.execute(&workflow).unwrap();

        assert!(result.duration_ms >= 100);
    }

    // =========================================================================
    // Parallel Execution Tests
    // =========================================================================

    #[test]
    fn test_parallel_execution_all_success() {
        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        let workflow = Workflow {
            steps: vec![
                Step::script("true"),
                Step::script("true"),
                Step::script("true"),
            ],
            execution: Some(ExecutionConfig {
                mode: ExecutionMode::Parallel,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = executor.execute(&workflow).unwrap();

        assert_eq!(result.state, WorkflowState::Success);
        assert_eq!(result.step_results.len(), 3);
        assert!(result.step_results.iter().all(|r| r.state == StepState::Success));
    }

    #[test]
    fn test_parallel_execution_maintains_order() {
        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        let workflow = Workflow {
            steps: vec![
                Step {
                    id: Some("step-a".to_string()),
                    run: Some("true".to_string()),
                    ..Step::script("")
                },
                Step {
                    id: Some("step-b".to_string()),
                    run: Some("true".to_string()),
                    ..Step::script("")
                },
                Step {
                    id: Some("step-c".to_string()),
                    run: Some("true".to_string()),
                    ..Step::script("")
                },
            ],
            execution: Some(ExecutionConfig {
                mode: ExecutionMode::Parallel,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = executor.execute(&workflow).unwrap();

        // Results should be in original step order
        assert_eq!(result.step_results[0].step_id, Some("step-a".to_string()));
        assert_eq!(result.step_results[1].step_id, Some("step-b".to_string()));
        assert_eq!(result.step_results[2].step_id, Some("step-c".to_string()));
    }

    #[test]
    fn test_parallel_execution_with_failure() {
        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        let workflow = Workflow {
            steps: vec![
                Step::script("true"),
                Step::script("exit 1"),
                Step::script("true"),
            ],
            execution: Some(ExecutionConfig {
                mode: ExecutionMode::Parallel,
                on_step_failure: OnStepFailure::Continue,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = executor.execute(&workflow).unwrap();

        assert_eq!(result.state, WorkflowState::Failed);
        assert_eq!(result.step_results.len(), 3);
        // All steps run in parallel, so all should have results
        assert_eq!(result.step_results[0].state, StepState::Success);
        assert_eq!(result.step_results[1].state, StepState::Failed);
        assert_eq!(result.step_results[2].state, StepState::Success);
    }

    #[test]
    fn test_parallel_execution_is_actually_parallel() {
        // Test that parallel execution is faster than sequential for sleep commands
        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        // 3 steps that each sleep 0.1s
        // Sequential would take ~0.3s, parallel should take ~0.1s
        let workflow = Workflow {
            steps: vec![
                Step::script("sleep 0.1"),
                Step::script("sleep 0.1"),
                Step::script("sleep 0.1"),
            ],
            execution: Some(ExecutionConfig {
                mode: ExecutionMode::Parallel,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = executor.execute(&workflow).unwrap();

        assert_eq!(result.state, WorkflowState::Success);
        // Should be significantly faster than 300ms (sequential time)
        // Allow some slack for test environment variance
        assert!(
            result.duration_ms < 250,
            "Parallel execution took {}ms, expected < 250ms",
            result.duration_ms
        );
    }

    #[test]
    fn test_parallel_with_max_parallel() {
        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        let workflow = Workflow {
            steps: vec![
                Step::script("true"),
                Step::script("true"),
                Step::script("true"),
            ],
            execution: Some(ExecutionConfig {
                mode: ExecutionMode::Parallel,
                max_parallel: Some(2), // Limit to 2 threads
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = executor.execute(&workflow).unwrap();

        assert_eq!(result.state, WorkflowState::Success);
        assert_eq!(result.step_results.len(), 3);
    }

    #[test]
    fn test_dag_batch_parallel_execution() {
        // Test that DAG batches execute in parallel
        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        // DAG: A -> B, C (B and C can run in parallel after A)
        let workflow = Workflow {
            steps: vec![
                Step {
                    id: Some("a".to_string()),
                    run: Some("true".to_string()),
                    depends: vec![],
                    ..Step::script("")
                },
                Step {
                    id: Some("b".to_string()),
                    run: Some("sleep 0.1".to_string()),
                    depends: vec!["a".to_string()],
                    ..Step::script("")
                },
                Step {
                    id: Some("c".to_string()),
                    run: Some("sleep 0.1".to_string()),
                    depends: vec!["a".to_string()],
                    ..Step::script("")
                },
            ],
            execution: Some(ExecutionConfig {
                mode: ExecutionMode::Dag,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = executor.execute(&workflow).unwrap();

        assert_eq!(result.state, WorkflowState::Success);
        // B and C should run in parallel (~0.1s), not sequential (~0.2s)
        assert!(
            result.duration_ms < 180,
            "DAG batch took {}ms, expected < 180ms (parallel b+c)",
            result.duration_ms
        );
    }

    // =========================================================================
    // OnError Tests
    // =========================================================================

    #[test]
    fn test_on_error_continue_overrides_workflow_abort() {
        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        // Workflow default is abort, but step has on_error: continue
        let mut step_fail = Step::script("exit 1");
        step_fail.on_error = Some(OnError::Continue);

        let workflow = Workflow {
            steps: vec![
                Step::script("true"),
                step_fail,
                Step::script("true"), // Should still run
            ],
            execution: Some(ExecutionConfig {
                on_step_failure: OnStepFailure::Abort,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = executor.execute(&workflow).unwrap();

        assert_eq!(result.state, WorkflowState::Failed);
        assert_eq!(result.step_results.len(), 3);
        assert_eq!(result.step_results[0].state, StepState::Success);
        assert_eq!(result.step_results[1].state, StepState::Failed);
        assert_eq!(result.step_results[2].state, StepState::Success); // Continues despite failure
    }

    #[test]
    fn test_on_error_fail_overrides_workflow_continue() {
        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        // Workflow default is continue, but step has on_error: fail
        let mut step_fail = Step::script("exit 1");
        step_fail.on_error = Some(OnError::Fail);

        let workflow = Workflow {
            steps: vec![
                Step::script("true"),
                step_fail,
                Step::script("true"), // Should NOT run
            ],
            execution: Some(ExecutionConfig {
                on_step_failure: OnStepFailure::Continue,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = executor.execute(&workflow).unwrap();

        assert_eq!(result.state, WorkflowState::Failed);
        assert_eq!(result.step_results.len(), 2); // Third step not executed
        assert_eq!(result.step_results[0].state, StepState::Success);
        assert_eq!(result.step_results[1].state, StepState::Failed);
    }

    #[test]
    fn test_on_error_inherit_uses_workflow_setting() {
        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        // No on_error set, should use workflow's on_step_failure
        let workflow = Workflow {
            steps: vec![
                Step::script("true"),
                Step::script("exit 1"),
                Step::script("true"),
            ],
            execution: Some(ExecutionConfig {
                on_step_failure: OnStepFailure::Continue,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = executor.execute(&workflow).unwrap();

        assert_eq!(result.step_results.len(), 3);
        assert_eq!(result.step_results[2].state, StepState::Success);
    }

    // =========================================================================
    // Retry Tests
    // =========================================================================

    #[test]
    fn test_retry_on_failure() {
        use crate::models::step::StepRetry;

        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        // Step with on_error: retry, max_attempts: 2
        // First attempt fails, second succeeds (we can't easily test this with scripts)
        // So we test that max retries are exhausted
        let mut step_fail = Step::script("exit 1");
        step_fail.on_error = Some(OnError::Retry);
        step_fail.retry = Some(StepRetry {
            max_attempts: 2,
            delay: Some("10ms".to_string()),
        });

        let workflow = Workflow {
            steps: vec![step_fail],
            execution: Some(ExecutionConfig {
                on_step_failure: OnStepFailure::Continue,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = executor.execute(&workflow).unwrap();

        assert_eq!(result.state, WorkflowState::Failed);
        assert_eq!(result.step_results.len(), 1);
        assert_eq!(result.step_results[0].state, StepState::Failed);
        // Should have attempted twice (attempt 0 and 1)
        assert_eq!(result.step_results[0].attempt, 1); // Last attempt index
    }

    #[test]
    fn test_retry_eventually_succeeds() {
        use crate::models::step::StepRetry;
        use std::fs;

        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        // Create a temp file to track attempts
        let temp_dir = std::env::temp_dir();
        let counter_file = temp_dir.join("wt_retry_test_counter");
        let _ = fs::remove_file(&counter_file);

        // Script that fails first time, succeeds second time
        let script = format!(
            r#"
            FILE="{}"
            if [ -f "$FILE" ]; then
                exit 0
            else
                touch "$FILE"
                exit 1
            fi
            "#,
            counter_file.display()
        );

        let mut step = Step::script(&script);
        step.on_error = Some(OnError::Retry);
        step.retry = Some(StepRetry {
            max_attempts: 3,
            delay: Some("10ms".to_string()),
        });

        let workflow = Workflow {
            steps: vec![step],
            ..Default::default()
        };

        let result = executor.execute(&workflow).unwrap();

        // Cleanup
        let _ = fs::remove_file(&counter_file);

        assert_eq!(result.state, WorkflowState::Success);
        assert_eq!(result.step_results[0].state, StepState::Success);
        assert_eq!(result.step_results[0].attempt, 1); // Second attempt (index 1)
    }

    #[test]
    fn test_no_retry_without_on_error_retry() {
        use crate::models::step::StepRetry;

        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        // Step with retry config but no on_error: retry - should NOT retry
        let mut step_fail = Step::script("exit 1");
        step_fail.retry = Some(StepRetry {
            max_attempts: 3,
            delay: None,
        });
        // on_error is None (Inherit)

        let workflow = Workflow {
            steps: vec![step_fail],
            execution: Some(ExecutionConfig {
                on_step_failure: OnStepFailure::Continue,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = executor.execute(&workflow).unwrap();

        assert_eq!(result.step_results[0].state, StepState::Failed);
        assert_eq!(result.step_results[0].attempt, 0); // Only one attempt
    }

    // =========================================================================
    // Resume Tests
    // =========================================================================

    #[test]
    fn test_resume_from_checkpoint() {
        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        // Workflow with 3 steps
        let workflow = Workflow {
            steps: vec![
                Step::script("true"),
                Step::script("true"),
                Step::script("true"),
            ],
            ..Default::default()
        };

        // Simulate saved results: first step succeeded, second failed
        let saved_results = vec![
            StepResult {
                step_id: None,
                state: StepState::Success,
                ..Default::default()
            },
            StepResult {
                step_id: None,
                state: StepState::Failed,
                ..Default::default()
            },
        ];

        let result = executor.resume(&workflow, saved_results).unwrap();

        assert_eq!(result.state, WorkflowState::Success);
        assert_eq!(result.step_results.len(), 3);
        // First step preserved
        assert_eq!(result.step_results[0].state, StepState::Success);
        // Second and third re-executed
        assert_eq!(result.step_results[1].state, StepState::Success);
        assert_eq!(result.step_results[2].state, StepState::Success);
    }

    #[test]
    fn test_resume_all_completed() {
        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        let workflow = Workflow {
            steps: vec![
                Step::script("true"),
                Step::script("true"),
            ],
            ..Default::default()
        };

        // All steps already succeeded
        let saved_results = vec![
            StepResult {
                state: StepState::Success,
                ..Default::default()
            },
            StepResult {
                state: StepState::Success,
                ..Default::default()
            },
        ];

        let result = executor.resume(&workflow, saved_results).unwrap();

        assert_eq!(result.state, WorkflowState::Success);
        assert_eq!(result.step_results.len(), 2);
        assert_eq!(result.duration_ms, 0); // No new execution
    }

    #[test]
    fn test_resume_from_beginning() {
        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        let workflow = Workflow {
            steps: vec![
                Step::script("true"),
                Step::script("true"),
            ],
            ..Default::default()
        };

        // First step failed, nothing to preserve
        let saved_results = vec![
            StepResult {
                state: StepState::Failed,
                ..Default::default()
            },
        ];

        let result = executor.resume(&workflow, saved_results).unwrap();

        assert_eq!(result.state, WorkflowState::Success);
        assert_eq!(result.step_results.len(), 2);
        // Both steps re-executed
        assert_eq!(result.step_results[0].state, StepState::Success);
        assert_eq!(result.step_results[1].state, StepState::Success);
    }

    #[test]
    fn test_resume_preserves_skipped() {
        let config = test_config();
        let context = test_context();
        let executor = WorkflowExecutor::new(&config, context);

        let workflow = Workflow {
            steps: vec![
                Step::script("true"),
                Step::script("true"),
                Step::script("true"),
            ],
            ..Default::default()
        };

        // First step succeeded, second skipped, third failed
        let saved_results = vec![
            StepResult {
                state: StepState::Success,
                ..Default::default()
            },
            StepResult {
                state: StepState::Skipped,
                ..Default::default()
            },
            StepResult {
                state: StepState::Failed,
                ..Default::default()
            },
        ];

        let result = executor.resume(&workflow, saved_results).unwrap();

        // First and second preserved, third re-executed
        assert_eq!(result.step_results.len(), 3);
        assert_eq!(result.step_results[0].state, StepState::Success);
        assert_eq!(result.step_results[1].state, StepState::Skipped);
        assert_eq!(result.step_results[2].state, StepState::Success);
    }
}
