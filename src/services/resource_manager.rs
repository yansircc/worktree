//! Resource management for task execution.
//!
//! Provides unified functions for:
//! - Stopping processes in multiplexer windows
//! - Cleaning up resources (window, worktree, branch)

use crate::error::Result;
use crate::models::{Instance, TaskState, WtConfig};
use crate::services::git;
use crate::services::multiplexer::create_multiplexer;

/// Stop the current process in a task's multiplexer window.
///
/// Sends Ctrl+C to the window to interrupt the running process.
/// Does nothing if the task has no window.
pub fn stop_process(config: &WtConfig, state: &TaskState) -> Result<()> {
    if let Some(instance) = &state.instance {
        if let Some(ref window) = instance.window_name {
            let mux = create_multiplexer(config.multiplexer_type());
            let _ = mux.send_keys(&instance.session_name, window, "C-c");
            println!("Stopped process in window '{}'", window);
        }
    }
    Ok(())
}

/// Stop the process for a specific instance.
///
/// Sends Ctrl+C to the window to interrupt the running process.
pub fn stop_instance_process(config: &WtConfig, instance: &Instance) -> Result<()> {
    if let Some(ref window) = instance.window_name {
        let mux = create_multiplexer(config.multiplexer_type());
        let _ = mux.send_keys(&instance.session_name, window, "C-c");
        println!("Stopped process in window '{}'", window);
    }
    Ok(())
}

/// Clean up resources for an instance (window, worktree, branch).
///
/// Order: window → worktree → branch (to avoid issues with cwd and refs)
pub fn cleanup_instance(config: &WtConfig, instance: &Instance) -> Result<()> {
    // Close window first
    if let Some(ref window) = instance.window_name {
        let mux = create_multiplexer(config.multiplexer_type());
        let _ = mux.kill_window(&instance.session_name, window);
    }

    // Remove worktree
    if let Some(ref path) = instance.worktree_path {
        let _ = git::remove_worktree(path);
    }

    // Delete branch (after worktree is removed)
    // Get repo root to run delete_branch from correct location
    if let Some(ref branch) = instance.branch {
        if let Ok(repo_root) = git::get_repo_root() {
            let _ = git::delete_branch(&repo_root, branch);
        }
    }

    Ok(())
}

/// Kill the multiplexer window for an instance if the flag is set.
///
/// Returns Ok(true) if window was killed, Ok(false) if not.
pub fn kill_window_if_requested(
    config: &WtConfig,
    instance: &Instance,
    kill_window: bool,
) -> Result<bool> {
    if !kill_window {
        return Ok(false);
    }

    if let Some(ref window) = instance.window_name {
        let mux = create_multiplexer(config.multiplexer_type());
        if mux.kill_window_if_exists(&instance.session_name, window)? {
            println!("Closed window '{}:{}'", instance.session_name, window);
            return Ok(true);
        }
    }

    Ok(false)
}
