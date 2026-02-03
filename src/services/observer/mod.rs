//! Observer module for Phases v2.
//!
//! Provides observation infrastructure for monitoring step/workflow execution.

pub mod log;
pub mod terminal;

pub use log::{create_workflow_log_entry, LogObserver};
pub use terminal::{TerminalObserver, TerminalSettings};
