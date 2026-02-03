//! Internal commands for wt.
//!
//! These commands are not intended for direct user use, but for use in
//! workflows. They provide atomic operations that can be composed together.

pub mod git;
pub mod misc;
pub mod mux;

use crate::error::{Result, WtError};

/// Execute an internal operation
pub fn execute(operation: String, args: Vec<String>) -> Result<()> {
    // Handle standalone operations (no colon)
    match operation.as_str() {
        "notify" | "confirm" | "abort" | "log" => {
            return misc::execute_notify(&operation, args);
        }
        _ => {}
    }

    // Parse operation format: "category:action"
    let parts: Vec<&str> = operation.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(WtError::InvalidInput(format!(
            "Invalid operation format '{}'. Expected 'category:action' (e.g., 'mux:focus-window', 'git:fetch')",
            operation
        )));
    }

    let category = parts[0];
    let action = parts[1];

    match category {
        "mux" => mux::execute(action, args),
        "git" => git::execute(action, args),
        "files" => misc::execute_files(action, args),
        "status" => misc::execute_status(action, args),
        "task" => misc::execute_task(action, args),
        "config" => misc::execute_config(action, args),
        _ => Err(WtError::InvalidInput(format!(
            "Unknown category '{}'. Available: mux, git, files, status, task, config",
            category
        ))),
    }
}
