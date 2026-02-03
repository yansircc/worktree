//! Workflow model for Phases v2 system.
//!
//! Workflow orchestrates multiple Steps with execution strategies:
//! - Sequential: steps run one after another
//! - Parallel: steps run concurrently
//! - DAG: steps run based on dependency graph

use serde::{Deserialize, Serialize};

use super::step::{Step, StepResult, StepState};

// ============================================================================
// WorkflowState
// ============================================================================

/// Workflow execution state (derived from step states)
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowState {
    /// Not started
    #[default]
    Pending,
    /// At least one step is running
    Running,
    /// All steps completed successfully (or skipped)
    Success,
    /// At least one step failed
    Failed,
    /// At least one step is blocked
    Blocked,
}

impl WorkflowState {
    /// Derive workflow state from step results
    pub fn derive(steps: &[StepResult]) -> Self {
        if steps.is_empty() {
            return WorkflowState::Pending;
        }

        // Check in priority order
        if steps.iter().any(|s| s.state == StepState::Running) {
            WorkflowState::Running
        } else if steps.iter().any(|s| s.state == StepState::Failed || s.state == StepState::Timeout) {
            WorkflowState::Failed
        } else if steps.iter().any(|s| s.state == StepState::Blocked) {
            WorkflowState::Blocked
        } else if steps
            .iter()
            .all(|s| matches!(s.state, StepState::Success | StepState::Skipped))
        {
            WorkflowState::Success
        } else {
            // Some steps still pending
            WorkflowState::Pending
        }
    }

    /// Check if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, WorkflowState::Success | WorkflowState::Failed | WorkflowState::Blocked)
    }

    /// Check if workflow completed successfully
    pub fn is_success(&self) -> bool {
        matches!(self, WorkflowState::Success)
    }

    /// Get display icon
    pub fn icon(&self) -> &'static str {
        match self {
            WorkflowState::Pending => "○",
            WorkflowState::Running => "●",
            WorkflowState::Success => "✓",
            WorkflowState::Failed => "✗",
            WorkflowState::Blocked => "⊘",
        }
    }
}

// ============================================================================
// Execution Mode
// ============================================================================

/// Workflow execution mode
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Steps run one after another
    #[default]
    Sequential,
    /// Steps run concurrently
    Parallel,
    /// Steps run based on dependency graph
    Dag,
}

// ============================================================================
// Failure Handling
// ============================================================================

/// Action to take when a step fails
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OnStepFailure {
    /// Abort the entire workflow
    #[default]
    Abort,
    /// Continue with remaining steps
    Continue,
    /// Skip steps that depend on failed step
    SkipDependents,
}

/// Action to take when a step is blocked
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OnStepBlocked {
    /// Pause the workflow (wait for intervention)
    #[default]
    Pause,
    /// Abort the entire workflow
    Abort,
    /// Continue with remaining steps
    Continue,
}

// ============================================================================
// Retry Config
// ============================================================================

/// Workflow-level retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRetry {
    /// Maximum retry attempts for failed steps
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    /// Delay between retries (e.g., "30s")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay: Option<String>,
}

fn default_max_attempts() -> u32 {
    2
}

impl Default for WorkflowRetry {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
            delay: None,
        }
    }
}

// ============================================================================
// Execution Config
// ============================================================================

/// Workflow execution configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Execution mode
    #[serde(default)]
    pub mode: ExecutionMode,
    /// Action on step failure
    #[serde(default)]
    pub on_step_failure: OnStepFailure,
    /// Action on step blocked
    #[serde(default)]
    pub on_step_blocked: OnStepBlocked,
    /// Retry configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<WorkflowRetry>,
}

// ============================================================================
// Context Config
// ============================================================================

/// How context is passed between steps
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextPassing {
    /// Accumulate all step outputs
    #[default]
    Accumulate,
    /// Only pass last step's output
    LastOnly,
    /// Explicit context specification in each step
    Explicit,
}

/// Workflow context configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextConfig {
    /// Shared variables available to all steps
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub shared: std::collections::HashMap<String, String>,
    /// How context is passed between steps
    #[serde(default)]
    pub passing: ContextPassing,
}

// ============================================================================
// Observe Config
// ============================================================================

/// Workflow observation configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowObserve {
    /// Show progress indicator
    #[serde(default)]
    pub progress: bool,
    /// Show summary on completion
    #[serde(default)]
    pub summary: bool,
}

// ============================================================================
// Workflow
// ============================================================================

/// A workflow containing multiple steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Workflow identifier (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Human-readable name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Steps to execute
    pub steps: Vec<Step>,

    /// Execution configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<ExecutionConfig>,

    /// Context configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<ContextConfig>,

    /// Observation configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observe: Option<WorkflowObserve>,
}

impl Default for Workflow {
    fn default() -> Self {
        Self {
            id: None,
            name: None,
            steps: Vec::new(),
            execution: None,
            context: None,
            observe: None,
        }
    }
}

impl Workflow {
    /// Create a new workflow with steps
    pub fn new(steps: Vec<Step>) -> Self {
        Self {
            steps,
            ..Default::default()
        }
    }

    /// Create a simple sequential workflow from scripts
    pub fn from_scripts(commands: &[&str]) -> Self {
        Self {
            steps: commands.iter().map(|cmd| Step::script(*cmd)).collect(),
            ..Default::default()
        }
    }

    /// Get execution mode
    pub fn execution_mode(&self) -> ExecutionMode {
        self.execution
            .as_ref()
            .map(|e| e.mode.clone())
            .unwrap_or_default()
    }

    /// Check if workflow is empty
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Get number of steps
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Get workflow display name
    pub fn display_name(&self) -> String {
        if let Some(name) = &self.name {
            name.clone()
        } else if let Some(id) = &self.id {
            id.clone()
        } else {
            format!("{} steps", self.steps.len())
        }
    }

    /// Build execution order based on mode
    pub fn execution_order(&self) -> Vec<Vec<usize>> {
        match self.execution_mode() {
            ExecutionMode::Sequential => {
                // Each step in its own batch
                (0..self.steps.len()).map(|i| vec![i]).collect()
            }
            ExecutionMode::Parallel => {
                // All steps in one batch
                vec![(0..self.steps.len()).collect()]
            }
            ExecutionMode::Dag => {
                // Topological sort based on dependencies
                self.topological_sort()
            }
        }
    }

    /// Topological sort for DAG execution
    fn topological_sort(&self) -> Vec<Vec<usize>> {
        use std::collections::{HashMap, HashSet};

        // Build step index map
        let step_indices: HashMap<&str, usize> = self
            .steps
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.id.as_ref().map(|id| (id.as_str(), i)))
            .collect();

        // Build dependency graph
        let mut in_degree: Vec<usize> = vec![0; self.steps.len()];
        let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); self.steps.len()];

        for (i, step) in self.steps.iter().enumerate() {
            for dep in &step.depends {
                if let Some(&dep_idx) = step_indices.get(dep.as_str()) {
                    in_degree[i] += 1;
                    dependents[dep_idx].push(i);
                }
            }
        }

        // Kahn's algorithm
        let mut result: Vec<Vec<usize>> = Vec::new();
        let mut processed: HashSet<usize> = HashSet::new();

        loop {
            // Find all steps with no unprocessed dependencies
            let ready: Vec<usize> = (0..self.steps.len())
                .filter(|&i| !processed.contains(&i) && in_degree[i] == 0)
                .collect();

            if ready.is_empty() {
                break;
            }

            // Add ready steps as a batch
            result.push(ready.clone());

            // Mark as processed and update in-degrees
            for i in ready {
                processed.insert(i);
                for &dep_idx in &dependents[i] {
                    in_degree[dep_idx] = in_degree[dep_idx].saturating_sub(1);
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_state_derive_empty() {
        let state = WorkflowState::derive(&[]);
        assert_eq!(state, WorkflowState::Pending);
    }

    #[test]
    fn test_workflow_state_derive_running() {
        let results = vec![
            StepResult {
                state: StepState::Success,
                ..Default::default()
            },
            StepResult {
                state: StepState::Running,
                ..Default::default()
            },
        ];
        assert_eq!(WorkflowState::derive(&results), WorkflowState::Running);
    }

    #[test]
    fn test_workflow_state_derive_failed() {
        let results = vec![
            StepResult {
                state: StepState::Success,
                ..Default::default()
            },
            StepResult {
                state: StepState::Failed,
                ..Default::default()
            },
        ];
        assert_eq!(WorkflowState::derive(&results), WorkflowState::Failed);
    }

    #[test]
    fn test_workflow_state_derive_blocked() {
        let results = vec![
            StepResult {
                state: StepState::Success,
                ..Default::default()
            },
            StepResult {
                state: StepState::Blocked,
                ..Default::default()
            },
        ];
        assert_eq!(WorkflowState::derive(&results), WorkflowState::Blocked);
    }

    #[test]
    fn test_workflow_state_derive_success() {
        let results = vec![
            StepResult {
                state: StepState::Success,
                ..Default::default()
            },
            StepResult {
                state: StepState::Skipped,
                ..Default::default()
            },
        ];
        assert_eq!(WorkflowState::derive(&results), WorkflowState::Success);
    }

    #[test]
    fn test_workflow_from_scripts() {
        let workflow = Workflow::from_scripts(&["npm install", "npm test"]);
        assert_eq!(workflow.len(), 2);
        assert!(workflow.steps[0].is_script());
    }

    #[test]
    fn test_workflow_execution_order_sequential() {
        let workflow = Workflow::from_scripts(&["a", "b", "c"]);
        let order = workflow.execution_order();
        assert_eq!(order.len(), 3);
        assert_eq!(order[0], vec![0]);
        assert_eq!(order[1], vec![1]);
        assert_eq!(order[2], vec![2]);
    }

    #[test]
    fn test_workflow_execution_order_parallel() {
        let mut workflow = Workflow::from_scripts(&["a", "b", "c"]);
        workflow.execution = Some(ExecutionConfig {
            mode: ExecutionMode::Parallel,
            ..Default::default()
        });
        let order = workflow.execution_order();
        assert_eq!(order.len(), 1);
        assert_eq!(order[0], vec![0, 1, 2]);
    }

    #[test]
    fn test_workflow_execution_order_dag() {
        let mut workflow = Workflow {
            steps: vec![
                Step {
                    id: Some("install".to_string()),
                    run: Some("npm install".to_string()),
                    depends: vec![],
                    ..Step::script("")
                },
                Step {
                    id: Some("lint".to_string()),
                    run: Some("npm run lint".to_string()),
                    depends: vec!["install".to_string()],
                    ..Step::script("")
                },
                Step {
                    id: Some("test".to_string()),
                    run: Some("npm test".to_string()),
                    depends: vec!["install".to_string()],
                    ..Step::script("")
                },
                Step {
                    id: Some("build".to_string()),
                    run: Some("npm run build".to_string()),
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

        let order = workflow.execution_order();
        // install first
        assert_eq!(order[0], vec![0]);
        // lint and test can run in parallel
        assert!(order[1].contains(&1) && order[1].contains(&2));
        // build last
        assert_eq!(order[2], vec![3]);
    }

    #[test]
    fn test_workflow_serialize() {
        let workflow = Workflow::from_scripts(&["npm test"]);
        let json = serde_json::to_string(&workflow).unwrap();
        assert!(json.contains("npm test"));

        let parsed: Workflow = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 1);
    }
}
