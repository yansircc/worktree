//! Resource management for task execution.
//!
//! Provides unified functions for:
//! - Stopping processes in multiplexer windows
//! - Allocating resources (branch, worktree, window)
//! - Cleaning up resources (window, worktree)

use crate::error::{Result, WtError};
use crate::models::phase::PhaseResources;
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

/// Clean up resources for an instance (window and worktree).
///
/// Order: window → worktree (to avoid issues with cwd)
/// Does not delete the branch (branch cleanup is handled separately).
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

    Ok(())
}

/// Allocate resources for a task based on phase requirements.
///
/// Creates branch, worktree, and/or multiplexer window as specified
/// by the phase's resource requirements.
pub fn allocate_resources(
    config: &WtConfig,
    task_name: &str,
    resources: &PhaseResources,
) -> Result<Instance> {
    let repo_root = git::get_repo_root()?;

    let mut instance = Instance {
        branch: None,
        worktree_path: None,
        session_name: config.session_name.clone(),
        window_name: None,
        session_id: None,
        multiplexer: config.multiplexer_type(),
    };

    // Create branch if needed
    if resources.branch {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() % 0xFFFFFF)
            .unwrap_or(0);
        let branch_name = format!("wt/{}-{:06x}", task_name, timestamp);
        instance.branch = Some(branch_name);
    }

    // Create worktree if needed (requires branch)
    if resources.worktree {
        let branch_name = instance
            .branch
            .as_ref()
            .ok_or_else(|| WtError::InvalidInput("worktree requires branch".into()))?;

        let worktree_path = format!("{}/{}", config.worktree_dir, task_name);
        let full_worktree_path = if worktree_path.starts_with('/') {
            worktree_path.clone()
        } else {
            format!("{}/{}", repo_root, worktree_path)
        };

        git::create_worktree(branch_name, &full_worktree_path)?;
        println!("Created worktree at {}", full_worktree_path);
        instance.worktree_path = Some(full_worktree_path);
    }

    // Create multiplexer window if needed
    if resources.window {
        let cwd = instance.worktree_path.as_deref().unwrap_or(".");
        let mux = create_multiplexer(config.multiplexer_type());
        let session_name = &config.session_name;

        if !mux.session_exists(session_name) {
            mux.create_session(session_name)?;
        }

        mux.create_window(session_name, task_name, cwd, "")?;
        println!(
            "Created window '{}' in session '{}'",
            task_name, session_name
        );
        instance.window_name = Some(task_name.to_string());
    }

    Ok(instance)
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
