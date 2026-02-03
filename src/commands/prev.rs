//! Prev command for Phases v2.
//!
//! Forces a task to go back to the previous phase.
//!
//! Usage: `wt prev <task>`
//!
//! Behavior:
//! 1. Stops current process (if any)
//! 2. Updates phase to previous
//! 3. Does NOT execute on_enter (rollback doesn't trigger workflow)

use crate::error::{Result, WtError};
use crate::models::{StatusStore, TaskPhase, TaskStatus, TaskStore, WtConfig};
use crate::services::multiplexer::create_multiplexer;

/// Execute the prev command
///
/// # Arguments
/// * `task_ref` - Task name or index
pub fn execute(task_ref: String) -> Result<()> {
    let store = TaskStore::load()?;
    let config = WtConfig::load()?;

    // Resolve task reference
    let task_name = store.resolve_task_ref(&task_ref)?;

    // Get current state
    let mut status_store = StatusStore::load()?;
    let state = status_store.get(&task_name);

    // Check current phase - must have a phase to go back from
    let current_phase = &state.phase;
    let prev_phase = get_previous_phase(current_phase);

    match prev_phase {
        None => {
            return Err(WtError::InvalidInput(format!(
                "Task '{}' is in phase '{}' which has no previous phase",
                task_name,
                current_phase.display_name()
            )));
        }
        Some(new_phase) => {
            // Stop current process if running
            if state.status == TaskStatus::Active {
                if let Some(instance) = &state.instance {
                    let mux = create_multiplexer(config.multiplexer_type());
                    // Send Ctrl+C to stop the process
                    let _ = mux.send_keys(
                        &instance.session_name,
                        &instance.window_name,
                        "C-c",
                    );
                    println!(
                        "Stopped process in window '{}'",
                        instance.window_name
                    );
                }
            }

            // Update state
            let task_state = status_store.get_mut(&task_name);
            task_state.phase = new_phase.clone();
            task_state.status = TaskStatus::Idle;
            task_state.idle_reason = None;
            task_state.active_since = None;

            // Save status
            status_store.save()?;

            println!(
                "Task '{}' moved back to phase '{}'",
                task_name,
                new_phase.display_name()
            );
            println!("Note: on_enter workflow was NOT executed (rollback mode)");
        }
    }

    Ok(())
}

/// Get the previous phase for a given phase
fn get_previous_phase(current: &TaskPhase) -> Option<TaskPhase> {
    // Standard phase sequence: None -> Developing -> Reviewing -> Merging
    // Prev moves backwards
    match current {
        TaskPhase::None => None,
        TaskPhase::Developing => Some(TaskPhase::None),
        TaskPhase::Reviewing => Some(TaskPhase::Developing),
        TaskPhase::Merging => Some(TaskPhase::Reviewing),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_previous_phase() {
        assert_eq!(get_previous_phase(&TaskPhase::None), None);
        assert_eq!(
            get_previous_phase(&TaskPhase::Developing),
            Some(TaskPhase::None)
        );
        assert_eq!(
            get_previous_phase(&TaskPhase::Reviewing),
            Some(TaskPhase::Developing)
        );
        assert_eq!(
            get_previous_phase(&TaskPhase::Merging),
            Some(TaskPhase::Reviewing)
        );
    }
}
