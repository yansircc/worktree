//! Internal commands for hooks.
//!
//! These commands are not intended for direct user use, but for use in hooks
//! scripts. They provide atomic operations that can be composed together.

pub mod git;
pub mod mux;

use crate::error::{Result, WtError};

/// Execute an internal operation
pub fn execute(operation: String, args: Vec<String>) -> Result<()> {
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
        _ => Err(WtError::InvalidInput(format!(
            "Unknown category '{}'. Available: mux, git",
            category
        ))),
    }
}
