//! Workflow executor for Phases v2.
//!
//! Orchestrates step execution with support for:
//! - Sequential execution
//! - Parallel execution
//! - DAG-based execution
//! - Observer integration (terminal progress, file logging)

use std::path::PathBuf;

use chrono::Utc;

use crate::error::Result;
use crate::models::step::{StepResult, StepState};
use crate::models::workflow::{ExecutionMode, OnStepBlocked, OnStepFailure, Workflow, WorkflowState};
use crate::models::WtConfig;
use crate::services::executor::context::ExecutionContext;
use crate::services::executor::step::StepExecutor;
use crate::services::observer::log::{create_workflow_log_entry, LogObserver};
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

        let mut log_observer = if let Some(ref dir) = self.log_dir {
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

        let step_results = match mode {
            ExecutionMode::Sequential => {
                self.execute_sequential(workflow, &on_failure, &on_blocked, &terminal_observer, &mut log_observer)
            }
            ExecutionMode::Parallel => {
                self.execute_parallel(workflow, &on_failure, &on_blocked, &terminal_observer, &mut log_observer)
            }
            ExecutionMode::Dag => {
                self.execute_dag(workflow, &on_failure, &on_blocked, &terminal_observer, &mut log_observer)
            }
        };

        let state = WorkflowState::derive(&step_results);
        let duration_ms = start.elapsed().as_millis() as u64;

        // Notify workflow complete
        if let Some(ref obs) = terminal_observer {
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

    /// Execute steps sequentially.
    fn execute_sequential(
        &self,
        workflow: &Workflow,
        on_failure: &OnStepFailure,
        on_blocked: &OnStepBlocked,
        terminal_observer: &Option<TerminalObserver>,
        log_observer: &mut Option<LogObserver>,
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

            let step_name = step.id.clone().unwrap_or_else(|| format!("step-{}", index));

            // Notify step start
            if let Some(ref obs) = terminal_observer {
                obs.on_step_start(index, &step_name);
            }
            if let Some(ref mut obs) = log_observer {
                let _ = obs.on_step_start(index, step.id.as_deref());
            }

            // Execute step
            let mut executor = StepExecutor::new(self.config, context.clone());
            if let Some(ref dir) = self.log_dir {
                executor = executor.with_log_dir(dir.clone());
            }
            let result = executor.execute(step);

            // Notify step complete
            if let Some(ref obs) = terminal_observer {
                obs.on_step_complete(index, &step_name, &result.state);
            }
            if let Some(ref mut obs) = log_observer {
                let _ = obs.on_step_complete(&result);
            }

            // Check for early termination
            let should_abort = match result.state {
                StepState::Failed | StepState::Timeout => {
                    matches!(on_failure, OnStepFailure::Abort)
                }
                StepState::Blocked => {
                    matches!(on_blocked, OnStepBlocked::Abort | OnStepBlocked::Pause)
                }
                _ => false,
            };

            // Store step output for later steps
            if let Some(ref id) = result.step_id {
                if result.state == StepState::Success {
                    // TODO: read actual output from file
                    context.step_outputs.insert(id.clone(), String::new());
                }
            }

            results.push(result);

            if should_abort {
                break;
            }
        }

        results
    }

    /// Execute steps in parallel.
    fn execute_parallel(
        &self,
        workflow: &Workflow,
        _on_failure: &OnStepFailure,
        _on_blocked: &OnStepBlocked,
        terminal_observer: &Option<TerminalObserver>,
        log_observer: &mut Option<LogObserver>,
    ) -> Vec<StepResult> {
        // For now, execute sequentially but mark as parallel
        // TODO: Use thread pool for true parallel execution
        let mut results: Vec<StepResult> = Vec::new();

        for (index, step) in workflow.steps.iter().enumerate() {
            let step_name = step.id.clone().unwrap_or_else(|| format!("step-{}", index));

            // Notify step start
            if let Some(ref obs) = terminal_observer {
                obs.on_step_start(index, &step_name);
            }
            if let Some(ref mut obs) = log_observer {
                let _ = obs.on_step_start(index, step.id.as_deref());
            }

            let context = self.context.clone().next_step(index, step.id.as_deref());
            let mut executor = StepExecutor::new(self.config, context);
            if let Some(ref dir) = self.log_dir {
                executor = executor.with_log_dir(dir.clone());
            }
            let result = executor.execute(step);

            // Notify step complete
            if let Some(ref obs) = terminal_observer {
                obs.on_step_complete(index, &step_name, &result.state);
            }
            if let Some(ref mut obs) = log_observer {
                let _ = obs.on_step_complete(&result);
            }

            results.push(result);
        }

        results
    }

    /// Execute steps based on DAG.
    fn execute_dag(
        &self,
        workflow: &Workflow,
        on_failure: &OnStepFailure,
        on_blocked: &OnStepBlocked,
        terminal_observer: &Option<TerminalObserver>,
        log_observer: &mut Option<LogObserver>,
    ) -> Vec<StepResult> {
        let mut results: Vec<Option<StepResult>> = vec![None; workflow.steps.len()];
        let execution_order = workflow.execution_order();

        for batch in execution_order {
            // Check if any dependency failed (for skip_dependents mode)
            let skip_indices: std::collections::HashSet<usize> = if matches!(on_failure, OnStepFailure::SkipDependents) {
                batch.iter()
                    .filter(|&&idx| {
                        // Check if any dependency failed
                        workflow.steps[idx].depends.iter().any(|dep_id| {
                            workflow.steps.iter().enumerate().any(|(i, s)| {
                                s.id.as_deref() == Some(dep_id.as_str())
                                    && results[i].as_ref().map(|r| r.state == StepState::Failed).unwrap_or(false)
                            })
                        })
                    })
                    .cloned()
                    .collect()
            } else {
                std::collections::HashSet::new()
            };

            // Execute batch (could be parallelized)
            for &idx in &batch {
                let step = &workflow.steps[idx];
                let step_name = step.id.clone().unwrap_or_else(|| format!("step-{}", idx));

                if skip_indices.contains(&idx) {
                    let result = StepResult {
                        step_id: workflow.steps[idx].id.clone(),
                        state: StepState::Skipped,
                        message: Some("Dependency failed".to_string()),
                        ..Default::default()
                    };

                    // Notify skipped step
                    if let Some(ref obs) = terminal_observer {
                        obs.on_step_complete(idx, &step_name, &result.state);
                    }

                    results[idx] = Some(result);
                    continue;
                }

                // Notify step start
                if let Some(ref obs) = terminal_observer {
                    obs.on_step_start(idx, &step_name);
                }
                if let Some(ref mut obs) = log_observer {
                    let _ = obs.on_step_start(idx, step.id.as_deref());
                }

                let context = self.context.clone().next_step(idx, step.id.as_deref());
                let mut executor = StepExecutor::new(self.config, context);
                if let Some(ref dir) = self.log_dir {
                    executor = executor.with_log_dir(dir.clone());
                }
                let result = executor.execute(step);

                // Notify step complete
                if let Some(ref obs) = terminal_observer {
                    obs.on_step_complete(idx, &step_name, &result.state);
                }
                if let Some(ref mut obs) = log_observer {
                    let _ = obs.on_step_complete(&result);
                }

                // Check for abort
                let should_abort = match result.state {
                    StepState::Failed | StepState::Timeout => {
                        matches!(on_failure, OnStepFailure::Abort)
                    }
                    StepState::Blocked => {
                        matches!(on_blocked, OnStepBlocked::Abort | OnStepBlocked::Pause)
                    }
                    _ => false,
                };

                results[idx] = Some(result);

                if should_abort {
                    // Fill remaining with pending
                    for r in results.iter_mut() {
                        if r.is_none() {
                            *r = Some(StepResult::default());
                        }
                    }
                    return results.into_iter().flatten().collect();
                }
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
}
