pub mod claude;
pub mod command;
pub mod dependency;
pub mod files;
pub mod git;
pub mod multiplexer;
pub mod task_context;
pub mod transcript;
pub mod workspace;

// Phases v2 modules
pub mod executor;
pub mod observer;

pub use task_context::TaskContext;
