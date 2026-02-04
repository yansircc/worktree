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
//! 5. Cleans up resources if returning to first phase (sequence[0])
//! 6. Does NOT execute on_enter (rollback doesn't trigger entry workflow)

use std::path::PathBuf;

use crate::error::{Result, WtError};
use crate::models::phase::{ExitReason, Phase};
use crate::models::state::TaskRuntimeState;
use crate::models::{Instance, TaskStatus, WtConfig};
use crate::services::executor::{prev_phase, ExecutionContext, PhaseTransition};
use crate::services::git;
use crate::services::resource_manager;
use crate::services::TaskContext;

/// Execute the prev command
///
/// # Arguments
/// * `task_ref` - Task name or index
pub fn execute(task_ref: String) -> Result<()> {
    let mut ctx = TaskContext::load(&task_ref)?;
    let task_name = ctx.name().to_string();

    // Get current state (clone for immutable access)
    let state = ctx.store.status.get(&task_name).clone();

    // Get phase sequence from config
    let phase_sequence = ctx.config.phase_sequence()?;

    // Get current phase_id from state
    let current_phase_id = state.phase.as_deref();

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
                resource_manager::stop_process(&ctx.config, &state)?;
            }

            // Get current phase definition and execute on_exit if configured
            if let Some(current_id) = current_phase_id {
                if let Some(current_phase) = ctx.config.get_phase(current_id) {
                    if current_phase.on_exit.is_some() {
                        execute_on_exit(
                            &ctx.config,
                            &task_name,
                            current_phase,
                            state.instance.as_ref(),
                        )?;
                    }
                }
            }

            // Check if prev_id is the first phase in sequence
            let is_first_phase = phase_sequence.first().map(|s| s.as_str()) == Some(prev_id);

            // Clean up resources if returning to first phase
            let instance = if is_first_phase && state.instance.is_some() {
                if let Some(ref inst) = state.instance {
                    resource_manager::cleanup_instance(&ctx.config, inst)?;
                    println!("Cleaned up resources (window, worktree, branch)");
                }
                None
            } else {
                state.instance.clone()
            };

            // Update state
            let task_state = ctx.state_mut();
            // If returning to first phase, go back to pending state
            if is_first_phase {
                task_state.phase = None;
                task_state.status = TaskStatus::Pending;
            } else {
                task_state.phase = Some(prev_id.to_string());
                task_state.status = TaskStatus::Idle;
            }
            task_state.step_result = None;
            task_state.active_since = None;
            task_state.instance = instance;

            // Save status
            ctx.save_status()?;

            println!("Task '{}' moved back to phase '{}'", task_name, prev_id);
            println!("Note: on_enter workflow was NOT executed (rollback mode)");

            Ok(())
        }
    }
}

/// Execute on_exit workflow for the current phase
fn execute_on_exit(
    config: &WtConfig,
    task_name: &str,
    phase: &Phase,
    instance: Option<&Instance>,
) -> Result<()> {
    let repo_root = git::get_repo_root()?;

    // Build execution context
    let default_branch = format!("wt/{}", task_name);
    let branch = instance
        .and_then(|i| i.branch.as_deref())
        .unwrap_or(&default_branch);
    let worktree = instance
        .and_then(|i| i.worktree_path.as_deref())
        .unwrap_or(&repo_root);
    let session = instance
        .map(|i| i.session_name.as_str())
        .unwrap_or(&config.session_name);
    let window = instance
        .and_then(|i| i.window_name.as_deref())
        .unwrap_or(task_name);

    let context = ExecutionContext::new(task_name, branch, worktree, &repo_root)
        .with_session(session)
        .with_window(window)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_phase_determines_pending() {
        use crate::models::project::PhasesConfig;
        use std::collections::HashMap;

        let mut definitions = HashMap::new();
        definitions.insert("pending".to_string(), Phase::new("pending"));
        definitions.insert("developing".to_string(), Phase::new("developing"));

        let mut config = WtConfig::default();
        config.phases = Some(PhasesConfig {
            sequence: vec!["pending".to_string(), "developing".to_string()],
            definitions,
        });

        // Verify first phase is "pending"
        let seq = config.phase_sequence().unwrap();
        assert_eq!(seq.first().map(|s| s.as_str()), Some("pending"));

        let pending = config.get_phase("pending").unwrap();
        assert!(!pending.terminal);

        let developing = config.get_phase("developing").unwrap();
        assert!(!developing.terminal);
    }
}
