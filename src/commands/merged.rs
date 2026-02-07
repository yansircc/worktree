use crate::error::{Result, WtError};
use crate::models::{TaskStatus, TaskStore};
use crate::services::tmux;

pub fn execute(task_ref: String, silent: bool) -> Result<()> {
    let mut store = TaskStore::load()?;

    // Resolve task reference (name or index) to actual name
    let name = store.resolve_task_ref(&task_ref)?;

    // Check if scratch environment
    if store.is_scratch(&name) {
        return Err(WtError::InvalidInput(format!(
            "Scratch environment '{}' cannot be marked as merged. Use 'wt archive {}' to clean up.",
            name, name
        )));
    }

    // Check task exists
    store.ensure_exists(&name)?;

    let current_status = store.get_status(&name);
    if !current_status.can_transition_to(&TaskStatus::Merged) && !silent {
        println!(
            "Warning: Task '{}' was in {} state (expected done or running).",
            name,
            current_status.display_name()
        );
    }

    // Only close tmux session, keep worktree and branch for review
    if let Some(instance) = store.get_instance(&name) {
        if let Err(e) = tmux::kill_session(&instance.tmux_session) {
            if !silent {
                eprintln!("  Warning: Failed to kill tmux session: {}", e);
            }
        } else if !silent {
            println!("  Closed tmux session: {}", instance.tmux_session);
        }
        // Keep instance data for archive command
    }

    store.set_status(&name, TaskStatus::Merged);
    // Keep instance (worktree_path, branch) for archive
    store.save_status()?;

    if !silent {
        println!("Task '{}' marked as merged.", name);
        println!("Worktree and branch preserved for review.");
        println!("Run 'wt archive {}' to cleanup resources.", name);
    }
    Ok(())
}
