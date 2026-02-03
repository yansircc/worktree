mod agent_step;
pub mod builtin_pipelines;
mod config;
mod status;
mod store;
mod task;
pub mod task_parser;

// Primary exports
pub use agent_step::AgentStep;
pub use config::{HookDef, Step, WtConfig, CONFIG_FILE};
#[allow(unused_imports)] // Public API - TaskState reserved for hooks state management
pub use status::{IdleReason, StatusStore, TaskPhase, TaskState, TaskStatus};
pub use store::TaskStore;
pub use task::{Instance, Task, TaskFrontmatter, TaskInput};
