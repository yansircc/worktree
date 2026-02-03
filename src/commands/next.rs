//! Next command for Phases v2.
//!
//! Forces a task to advance to the next phase.
//!
//! Usage: `wt next <task>`
//!
//! Behavior:
//! 1. Validates current phase allows transition
//! 2. Stops current process (if any)
//! 3. Updates phase to next
//! 4. Triggers on_enter workflow (if configured)

use crate::error::{Result, WtError};
use crate::models::{IdleReason, StatusStore, TaskPhase, TaskStatus, TaskStore, WtConfig};
use crate::services::multiplexer::create_multiplexer;

/// Execute the next command
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

    // Check if already completed first
    if state.status == TaskStatus::Completed {
        return Err(WtError::InvalidInput(format!(
            "Task '{}' is already completed",
            task_name
        )));
    }

    // Get current and next phase
    let current_phase = &state.phase;
    let next_phase = get_next_phase(current_phase);

    match next_phase {
        None => {
            // At final phase (merging), no next
            return Err(WtError::InvalidInput(format!(
                "Task '{}' is in phase '{}' which has no next phase. Use 'wt complete' to finish.",
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

            // Check if transitioning to "completed" (final phase)
            let is_completing = new_phase == TaskPhase::None && current_phase == &TaskPhase::Merging;

            // Update state
            let task_state = status_store.get_mut(&task_name);

            if is_completing {
                // Mark as completed
                task_state.status = TaskStatus::Completed;
                task_state.phase = TaskPhase::None;
                task_state.idle_reason = None;
                task_state.active_since = None;
                println!("Task '{}' marked as completed", task_name);
            } else {
                task_state.phase = new_phase.clone();
                // Set to Idle, waiting for next action
                task_state.status = TaskStatus::Idle;
                task_state.idle_reason = Some(IdleReason::Done);
                task_state.active_since = None;
                println!(
                    "Task '{}' advanced to phase '{}'",
                    task_name,
                    new_phase.display_name()
                );
            }

            // Save status
            status_store.save()?;

            // Note about on_enter workflow
            if !is_completing {
                println!("Hint: Run 'wt run {}' to start the {} workflow", task_name, new_phase.display_name());
            }
        }
    }

    Ok(())
}

/// Get the next phase for a given phase
fn get_next_phase(current: &TaskPhase) -> Option<TaskPhase> {
    // Standard phase sequence: None -> Developing -> Reviewing -> Merging -> (Completed)
    match current {
        TaskPhase::None => Some(TaskPhase::Developing),
        TaskPhase::Developing => Some(TaskPhase::Reviewing),
        TaskPhase::Reviewing => Some(TaskPhase::Merging),
        TaskPhase::Merging => None, // Completion handled separately
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_next_phase() {
        assert_eq!(
            get_next_phase(&TaskPhase::None),
            Some(TaskPhase::Developing)
        );
        assert_eq!(
            get_next_phase(&TaskPhase::Developing),
            Some(TaskPhase::Reviewing)
        );
        assert_eq!(
            get_next_phase(&TaskPhase::Reviewing),
            Some(TaskPhase::Merging)
        );
        assert_eq!(get_next_phase(&TaskPhase::Merging), None);
    }
}
