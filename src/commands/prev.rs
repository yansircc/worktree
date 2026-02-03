//! Prev command for Phases v2.
//!
//! Forces a task to go back to the previous phase.
//!
//! Usage: `wt prev <task>`
//!
//! Behavior:
//! 1. Determines previous phase from config sequence
//! 2. Stops current process (if any)
//! 3. Executes on_exit workflow (if configured)
//! 4. Updates phase to previous
//! 5. Cleans up resources if returning to pending
//! 6. Does NOT execute on_enter (rollback doesn't trigger entry workflow)

use std::path::PathBuf;

use crate::error::{Result, WtError};
use crate::models::phase::{ExitReason, Phase, PhaseResources};
use crate::models::state::TaskRuntimeState;
use crate::models::{StatusStore, TaskPhase, TaskStatus, TaskStore, WtConfig};
use crate::services::executor::{prev_phase, ExecutionContext, PhaseTransition};
use crate::services::git;
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

            // Get current phase definition and execute on_exit if configured
            if let Some(current_id) = current_phase_id {
                let current_phase = get_phase_definition(&config, current_id);
                if current_phase.on_exit.is_some() {
                    execute_on_exit(&config, &task_name, &current_phase, state.instance.as_ref())?;
                }
            }

            // Get previous phase definition to check resource requirements
            let prev_phase_def = get_phase_definition(&config, prev_id);
            let prev_needs_resources = prev_phase_def.resources == PhaseResources::Full;

            // Clean up resources if returning to a phase that doesn't need them
            let instance = if !prev_needs_resources && state.instance.is_some() {
                if let Some(ref inst) = state.instance {
                    cleanup_resources(&config, inst)?;
                    println!("Cleaned up resources (worktree, window)");
                }
                None
            } else {
                state.instance.clone()
            };

            // Update state
            let task_state = status_store.get_mut(&task_name);
            task_state.phase = phase_id_to_task_phase(prev_id);
            task_state.status = if prev_id == "pending" {
                TaskStatus::Pending
            } else {
                TaskStatus::Idle
            };
            task_state.idle_reason = None;
            task_state.active_since = None;
            task_state.instance = instance;

            // Save status
            status_store.save()?;

            println!("Task '{}' moved back to phase '{}'", task_name, prev_id);
            println!("Note: on_enter workflow was NOT executed (rollback mode)");

            Ok(())
        }
    }
}

/// Get phase definition from config or create default
fn get_phase_definition(config: &WtConfig, phase_id: &str) -> Phase {
    if let Some(phase) = config.get_phase(phase_id) {
        return phase.clone();
    }

    // Create default based on phase name
    match phase_id {
        "pending" | "completed" => Phase::new(phase_id),
        _ => Phase::with_resources(phase_id),
    }
}

/// Execute on_exit workflow for the current phase
fn execute_on_exit(
    config: &WtConfig,
    task_name: &str,
    phase: &Phase,
    instance: Option<&crate::models::Instance>,
) -> Result<()> {
    let repo_root = git::get_repo_root()?;

    // Build execution context
    let (branch, worktree, session, window) = if let Some(inst) = instance {
        (
            inst.branch.clone(),
            inst.worktree_path.clone(),
            inst.session_name.clone(),
            inst.window_name.clone(),
        )
    } else {
        (
            format!("wt/{}", task_name),
            repo_root.clone(),
            config.session_name.clone(),
            task_name.to_string(),
        )
    };

    let context = ExecutionContext::new(task_name, &branch, &worktree, &repo_root)
        .with_session(&session)
        .with_window(&window)
        .with_phase(&phase.id);

    // Create runtime state
    let mut runtime_state = TaskRuntimeState::pending();
    runtime_state.transition_to(&phase.id);

    // Execute phase exit with progress output
    let transition = PhaseTransition::new(config, context)
        .with_log_dir(PathBuf::from(".wt/logs"))
        .with_progress(true);

    let _result = transition.exit(phase, ExitReason::Forced, &mut runtime_state)?;

    Ok(())
}

/// Clean up resources (worktree, window)
fn cleanup_resources(config: &WtConfig, instance: &crate::models::Instance) -> Result<()> {
    // Remove worktree
    let _ = git::remove_worktree(&instance.worktree_path);

    // Close window
    let mux = create_multiplexer(config.multiplexer_type());
    let _ = mux.kill_window(&instance.session_name, &instance.window_name);

    Ok(())
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
