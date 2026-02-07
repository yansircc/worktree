use std::env;
use std::process::Command;

use crate::error::{Result, WtError};
use crate::models::{TaskStatus, TaskStore};
use crate::services::tmux;

pub fn execute(task_ref: String) -> Result<()> {
    let store = TaskStore::load()?;

    // Resolve task reference (name or index) to actual name
    let name = store.resolve_task_ref(&task_ref)?;

    // Check task exists in status.json
    if !store.name_exists_in_status(&name) {
        return Err(WtError::TaskNotFound(name));
    }

    let status = store.get_status(&name);

    // Only Running or Done tasks have tmux sessions
    if !matches!(status, TaskStatus::Running | TaskStatus::Done) {
        return Err(WtError::InvalidInput(format!(
            "Task '{}' is {} (need running or done to enter)",
            name,
            status.display_name()
        )));
    }

    // Get instance info
    let instance = store
        .get_instance(&name)
        .ok_or_else(|| WtError::InvalidInput(format!("Task '{}' has no instance info", name)))?;

    let session = &instance.tmux_session;

    // Check if tmux session exists
    if !tmux::session_exists(session) {
        return Err(WtError::InvalidInput(format!(
            "Tmux session '{}' not found. Task may have been stopped.",
            session
        )));
    }

    // Check if we're inside tmux
    let in_tmux = env::var("TMUX").is_ok();

    if in_tmux {
        // Inside tmux: switch client to target session
        let status = Command::new("tmux")
            .args(["switch-client", "-t", session])
            .status()
            .map_err(|e| WtError::Tmux(e.to_string()))?;

        if !status.success() {
            return Err(WtError::Tmux(format!(
                "Failed to switch to session '{}'",
                session
            )));
        }
    } else {
        // Outside tmux: attach to session
        let status = Command::new("tmux")
            .args(["attach", "-t", session])
            .status()
            .map_err(|e| WtError::Tmux(e.to_string()))?;

        if !status.success() {
            return Err(WtError::Tmux(format!(
                "Failed to attach to session '{}'",
                session
            )));
        }
    }

    Ok(())
}
