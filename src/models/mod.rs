mod agent_step;
pub mod builtin_pipelines;
mod config;
mod status;
mod store;
mod task;
pub mod task_parser;

// Phases v2 modules
pub mod phase;
pub mod project;
pub mod state;
pub mod step;
pub mod workflow;

// Primary exports (legacy)
pub use agent_step::AgentStep;
pub use config::{HookDef, Step, WtConfig, CONFIG_FILE};
#[allow(unused_imports)] // Public API - TaskState reserved for hooks state management
pub use status::{IdleReason, StatusStore, TaskPhase, TaskState, TaskStatus};
pub use store::TaskStore;
pub use task::{Instance, Task, TaskFrontmatter, TaskInput};

// Phases v2 exports
pub use phase::{Phase, PhaseResources, PhaseState};
pub use project::{Project, ProjectStatus};
pub use state::{DerivedTaskStatus, TaskRuntimeState};
pub use step::{StepResult, StepState};
pub use workflow::{ExecutionMode, Workflow, WorkflowState};
