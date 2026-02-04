//! Stop command for Phases v2.
//!
//! Stops a running task's process but keeps resources (worktree, branch).
//!
//! Usage: `wt stop <task>`
//!
//! Behavior:
//! 1. Sends Ctrl+C to the task's process
//! 2. Optionally closes the multiplexer window (with --kill-window)
//! 3. Sets task status to Idle with reason "manual"
//! 4. Keeps worktree and branch intact for resuming

use std::fs::OpenOptions;
use std::io::Write;

use chrono::Utc;

use crate::error::{Result, WtError};
use crate::models::{StepResult, TaskStatus};
use crate::services::resource_manager;
use crate::services::TaskContext;

/// Execute the stop command
///
/// # Arguments
/// * `task_ref` - Task name or index
/// * `kill_window` - Whether to also close the multiplexer window
pub fn execute(task_ref: String, kill_window: bool) -> Result<()> {
    let mut ctx = TaskContext::load(&task_ref)?;
    let task_name = ctx.name().to_string();

    // Get current state (clone for immutable access)
    let state = ctx.store.status.get(&task_name).clone();

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

    // Stop the process (sends Ctrl+C)
    resource_manager::stop_instance_process(&ctx.config, &instance)?;
    if instance.window_name.is_some() {
        println!(
            "Sent stop signal to window '{}:{}'",
            instance.session_name,
            instance.window_name.as_deref().unwrap()
        );
    }

    // Optionally close the window
    resource_manager::kill_window_if_requested(&ctx.config, &instance, kill_window)?;

    // Log stop event
    log_stop_event(&task_name, state.phase.as_deref(), kill_window);

    // Update state
    let task_state = ctx.state_mut();
    task_state.status = TaskStatus::Idle;
    task_state.step_result = Some(StepResult::Manual);
    task_state.active_since = None;
    // Keep instance info for resume

    // Save status
    ctx.save_status()?;

    println!("Task '{}' stopped.", task_name);
    println!(
        "Hint: Run 'wt next {}' to resume from phase '{}'",
        task_name,
        state.phase.as_deref().unwrap_or("none")
    );

    Ok(())
}

/// Log stop event to task log file
fn log_stop_event(task_name: &str, phase: Option<&str>, kill_window: bool) {
    let log_dir = format!(".wt/logs/{}", task_name);
    if std::fs::create_dir_all(&log_dir).is_err() {
        return;
    }

    let log_path = format!("{}/stop.log", log_dir);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        let _ = writeln!(
            file,
            "[{}] Stopped in phase '{}' (kill_window: {})",
            timestamp,
            phase.unwrap_or("none"),
            kill_window
        );
    }
}

#[cfg(test)]
mod tests {
    // Stop command requires a running task with multiplexer,
    // so unit tests are limited. Integration tests cover this.
}
