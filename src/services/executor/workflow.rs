//! Workflow executor for Phases v2.
//!
//! Orchestrates step execution with support for:
//! - Sequential execution
//! - Parallel execution (using rayon thread pool)
//! - DAG-based execution (batches run in parallel)
//! - Observer integration (terminal progress, file logging)

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::Utc;
use rayon::prelude::*;

use crate::error::Result;
use crate::models::step::{Step, StepResult, StepState};
use crate::models::workflow::{ExecutionMode, OnStepBlocked, OnStepFailure, Workflow, WorkflowState};
use crate::models::WtConfig;
use crate::services::executor::context::ExecutionContext;
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
            let phase_name = if self.context.phase.is_empty() {
                "unknown"
            } else {
                &self.context.phase
            };
            let mut obs = LogObserver::new(dir, &self.context.task, phase_name)
                .with_stream(true);
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
                self.execute_sequential(workflow, &on_failure, &on_blocked, &sync_observers)
            }
            ExecutionMode::Parallel => {
                self.execute_parallel(workflow, &on_failure, &on_blocked, max_parallel, &sync_observers)
            }
            ExecutionMode::Dag => {
                self.execute_dag(workflow, &on_failure, &on_blocked, max_parallel, &sync_observers)
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

    /// Check if execution should abort based on step result.
    fn should_abort(
        result: &StepResult,
        on_failure: &OnStepFailure,
        on_blocked: &OnStepBlocked,
    ) -> bool {
        match result.state {
            StepState::Failed | StepState::Timeout => {
                matches!(on_failure, OnStepFailure::Abort)
            }
            StepState::Blocked => {
                matches!(on_blocked, OnStepBlocked::Abort | OnStepBlocked::Pause)
            }
            _ => false,
        }
    }

    /// Execute steps sequentially.
    fn execute_sequential(
        &self,
        workflow: &Workflow,
        on_failure: &OnStepFailure,
        on_blocked: &OnStepBlocked,
        observers: &SyncObservers,
    ) -> Vec<StepResult> {
        let mut results: Vec<StepResult> = Vec::new();
        let mut context = self.context.clone();

        for (index, step) in workflow.steps.iter().enumerate() {
            // Update context for this step
            context = context.next_step(index, step.id.as_deref());

            // Add previous step info
            if let Some(prev) = results.last() {
                context.prev_state = Some(format!("{:?}", prev.state).to_lowercase());
            }

            let result = self.execute_step_with_sync_observers(
                step, index, context.clone(), observers
            );

            // Store step output for later steps
            if let Some(ref id) = result.step_id {
                if result.state == StepState::Success {
                    // TODO: read actual output from file
                    context.step_outputs.insert(id.clone(), String::new());
                }
            }

            let should_abort = Self::should_abort(&result, on_failure, on_blocked);
            results.push(result);

            if should_abort {
                break;
            }
        }

        results
    }

    /// Execute steps in parallel using rayon thread pool.
    ///
    /// All steps run concurrently with optional thread limit via `max_parallel`.
    /// Results are returned in original step order regardless of completion order.
    fn execute_parallel(
        &self,
        workflow: &Workflow,
        on_failure: &OnStepFailure,
        on_blocked: &OnStepBlocked,
        max_parallel: Option<usize>,
        observers: &SyncObservers,
    ) -> Vec<StepResult> {
        // Early abort flag for Abort mode
        let aborted = AtomicBool::new(false);

        // Build thread pool with optional parallelism limit
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(max_parallel.unwrap_or(0)) // 0 = use all available
            .build()
            .unwrap_or_else(|_| rayon::ThreadPoolBuilder::new().build().unwrap());

        pool.install(|| {
            workflow.steps
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

                    let context = self.context.clone().next_step(index, step.id.as_deref());
                    let result = self.execute_step_with_sync_observers(step, index, context, observers);

                    // Signal abort if needed
                    if Self::should_abort(&result, on_failure, on_blocked) {
                        aborted.store(true, Ordering::Relaxed);
                    }

                    result
                })
                .collect()
        })
    }

    /// Execute steps based on DAG with parallel batch execution.
    ///
    /// Steps are grouped into batches by dependency order. Within each batch,
    /// steps run in parallel. Batches execute sequentially (later batches wait
    /// for earlier ones to complete).
    fn execute_dag(
        &self,
        workflow: &Workflow,
        on_failure: &OnStepFailure,
        on_blocked: &OnStepBlocked,
        max_parallel: Option<usize>,
        observers: &SyncObservers,
    ) -> Vec<StepResult> {
        let mut results: Vec<Option<StepResult>> = vec![None; workflow.steps.len()];
        let execution_order = workflow.execution_order();

        // Build thread pool with optional parallelism limit
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(max_parallel.unwrap_or(0))
            .build()
            .unwrap_or_else(|_| rayon::ThreadPoolBuilder::new().build().unwrap());

        for batch in execution_order {
            // Check if any dependency failed (for skip_dependents mode)
            let skip_indices: std::collections::HashSet<usize> = if matches!(on_failure, OnStepFailure::SkipDependents) {
                batch.iter()
                    .filter(|&&idx| {
                        workflow.steps[idx].depends.iter().any(|dep_id| {
                            workflow.steps.iter().enumerate().any(|(i, s)| {
                                s.id.as_deref() == Some(dep_id.as_str())
                                    && results[i].as_ref().map(|r| r.state == StepState::Failed).unwrap_or(false)
                            })
                        })
                    })
                    .copied()
                    .collect()
            } else {
                std::collections::HashSet::new()
            };

            // Early abort flag
            let aborted = AtomicBool::new(false);

            // Execute batch in parallel
            let batch_results: Vec<(usize, StepResult)> = pool.install(|| {
                batch.par_iter()
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

                        let context = self.context.clone().next_step(idx, step.id.as_deref());
                        let result = self.execute_step_with_sync_observers(step, idx, context, observers);

                        // Signal abort if needed
                        if Self::should_abort(&result, on_failure, on_blocked) {
                            aborted.store(true, Ordering::Relaxed);
                        }

                        (idx, result)
                    })
                    .collect()
            });

            // Store batch results
            let mut should_abort_workflow = false;
            for (idx, result) in batch_results {
                if Self::should_abort(&result, on_failure, on_blocked) {
                    should_abort_workflow = true;
                }
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
}
