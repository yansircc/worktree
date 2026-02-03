pub mod claude;
pub mod command;
pub mod config_ops;
pub mod dependency;
pub mod files;
pub mod git;
pub mod hooks;
pub mod multiplexer;
pub mod notify;
pub mod status_ops;
pub mod task_context;
pub mod transcript;
pub mod workspace;

// Phases v2 modules
pub mod executor;
pub mod observer;

pub use task_context::TaskContext;
