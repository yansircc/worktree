use crate::error::{Result, WtError};
use crate::models::{TaskStatus, TaskStore};
use crate::services::multiplexer::create_multiplexer;

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

    // Close multiplexer window if still alive
    if let Some(instance) = store.get_instance(&name) {
        let mux = create_multiplexer(instance.multiplexer_type());
        if mux.kill_window_if_exists(&instance.session_name, &instance.window_name)? {
            println!(
                "Closed {} window {}:{}",
                instance.multiplexer, instance.session_name, instance.window_name
            );
        }
    }

    store.set_status(&name, TaskStatus::Done);
    store.save_status()?;

    println!("Task '{}' marked as done.", name);
    println!("To merge into main, run: wt merge {}", name);
    Ok(())
}
