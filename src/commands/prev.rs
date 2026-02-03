//! Prev command for Phases v2.
//!
//! Forces a task to go back to the previous phase.
//!
//! Usage: `wt prev <task>`
//!
//! Behavior:
//! 1. Determines previous phase from config sequence
//! 2. Stops current process (if any)
//! 3. Updates phase to previous
//! 4. Does NOT execute on_enter (rollback doesn't trigger workflow)

use crate::error::{Result, WtError};
use crate::models::{StatusStore, TaskPhase, TaskStatus, TaskStore, WtConfig};
use crate::services::executor::prev_phase;
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

    // Get phase sequence from config
    let phase_sequence = config.phase_sequence();

    // Convert current TaskPhase to phase_id string
    let current_phase_id = state.phase_id();

    // Determine previous phase
    let prev_phase_id = prev_phase(current_phase_id, &phase_sequence);

    match prev_phase_id {
        None => {
            let current_name = current_phase_id.unwrap_or("none");
            Err(WtError::InvalidInput(format!(
                "Task '{}' is in phase '{}' which has no previous phase",
                task_name, current_name
            )))
        }
        Some(prev_id) => {
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
            task_state.phase = phase_id_to_task_phase(prev_id);
            task_state.status = TaskStatus::Idle;
            task_state.idle_reason = None;
            task_state.active_since = None;

            // Save status
            status_store.save()?;

            println!("Task '{}' moved back to phase '{}'", task_name, prev_id);
            println!("Note: on_enter workflow was NOT executed (rollback mode)");

            Ok(())
        }
    }
}

/// Convert phase_id string to legacy TaskPhase enum
fn phase_id_to_task_phase(phase_id: &str) -> TaskPhase {
    match phase_id {
        "developing" => TaskPhase::Developing,
        "reviewing" => TaskPhase::Reviewing,
        "merging" => TaskPhase::Merging,
        _ => TaskPhase::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_id_to_task_phase() {
        assert_eq!(phase_id_to_task_phase("developing"), TaskPhase::Developing);
        assert_eq!(phase_id_to_task_phase("reviewing"), TaskPhase::Reviewing);
        assert_eq!(phase_id_to_task_phase("merging"), TaskPhase::Merging);
        assert_eq!(phase_id_to_task_phase("pending"), TaskPhase::None);
        assert_eq!(phase_id_to_task_phase("unknown"), TaskPhase::None);
    }
}
