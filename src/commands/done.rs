use crate::error::{Result, WtError};
use crate::models::{TaskStatus, TaskStore};
use crate::services::tmux;

pub fn execute(task_ref: String) -> Result<()> {
    let mut store = TaskStore::load()?;

    // Resolve task reference (name or index) to actual name
    let name = store.resolve_task_ref(&task_ref)?;

    // Check if scratch environment
    if store.is_scratch(&name) {
        return Err(WtError::InvalidInput(format!(
            "Scratch environment '{}' cannot be marked as done. Use 'wt archive {}' to clean up.",
            name, name
        )));
    }

    // Check task exists and validate transition
    store.ensure_exists(&name)?;
    store.validate_transition(&name, TaskStatus::Done)?;

    // Close tmux window if still alive
    if let Some(instance) = store.get_instance(&name) {
        if tmux::kill_window_if_exists(&instance.tmux_session, &instance.tmux_window)? {
            println!("Closed tmux window {}:{}", instance.tmux_session, instance.tmux_window);
        }
    }

    store.set_status(&name, TaskStatus::Done);
    store.save_status()?;

    println!("Task '{}' marked as done.", name);
    println!("After PR is merged, run: wt merged {}", name);
    Ok(())
}
