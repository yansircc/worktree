//! Stop command for Phases v2.
//!
//! Stops a running task's process but keeps resources (worktree, branch).
//!
//! Usage: `wt stop <task>`
//!
//! Behavior:
//! 1. Sends Ctrl+C to the task's process
//! 2. Optionally closes the multiplexer window (with --kill-window)
//! 3. Sets task status to Idle
//! 4. Keeps worktree and branch intact

use crate::error::{Result, WtError};
use crate::models::{IdleReason, StatusStore, TaskStatus, TaskStore, WtConfig};
use crate::services::multiplexer::create_multiplexer;

/// Execute the stop command
///
/// # Arguments
/// * `task_ref` - Task name or index
/// * `kill_window` - Whether to also close the multiplexer window
pub fn execute(task_ref: String, kill_window: bool) -> Result<()> {
    let store = TaskStore::load()?;
    let config = WtConfig::load()?;

    // Resolve task reference
    let task_name = store.resolve_task_ref(&task_ref)?;

    // Get current state
    let mut status_store = StatusStore::load()?;
    let state = status_store.get(&task_name);

    // Validate: must be Active
    if state.status != TaskStatus::Active {
        return Err(WtError::InvalidStateTransition {
            from: state.status.display_name().to_string(),
            to: "stop".to_string(),
        });
    }

    // Get instance info
    let instance = state.instance.clone().ok_or_else(|| {
        WtError::InvalidInput(format!(
            "Task '{}' has no instance (no worktree/window)",
            task_name
        ))
    })?;

    let mux = create_multiplexer(config.multiplexer_type());

    // Send Ctrl+C to stop the process
    let _ = mux.send_keys(&instance.session_name, &instance.window_name, "C-c");
    println!(
        "Sent stop signal to window '{}:{}'",
        instance.session_name, instance.window_name
    );

    // Optionally close the window
    if kill_window {
        if mux.kill_window_if_exists(&instance.session_name, &instance.window_name)? {
            println!(
                "Closed window '{}:{}'",
                instance.session_name, instance.window_name
            );
        }
    }

    // Update state
    let task_state = status_store.get_mut(&task_name);
    task_state.status = TaskStatus::Idle;
    task_state.idle_reason = Some(IdleReason::Manual);
    task_state.active_since = None;
    // Keep instance info for resume

    // Save status
    status_store.save()?;

    println!("Task '{}' stopped.", task_name);
    println!("Hint: Run 'wt run {}' to resume", task_name);

    Ok(())
}

#[cfg(test)]
mod tests {
    // Stop command requires a running task with multiplexer,
    // so unit tests are limited. Integration tests cover this.
}
