//! Next command for Phases v2.
//!
//! Forces a task to advance to the next phase.
//!
//! Usage: `wt next <task>`
//!
//! Behavior:
//! 1. Determines next phase from config sequence
//! 2. Stops current process (if any)
//! 3. Allocates resources if needed (worktree, branch, window)
//! 4. Executes on_enter workflow (if configured)
//! 5. Updates status

use std::path::PathBuf;

use chrono::Utc;

use crate::error::{Result, WtError};
use crate::models::phase::{Phase, PhaseResources};
use crate::models::state::TaskRuntimeState;
use crate::models::step::ObserveMode;
use crate::models::workflow::WorkflowState;
use crate::models::{IdleReason, Instance, StatusStore, TaskPhase, TaskStatus, TaskStore, WtConfig};
use crate::services::claude::ClaudeCommandBuilder;
use crate::services::executor::{next_phase, ExecutionContext, PhaseTransition};
use crate::services::git;
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

    // Check if already completed
    if state.status == TaskStatus::Completed {
        return Err(WtError::InvalidInput(format!(
            "Task '{}' is already completed",
            task_name
        )));
    }

    // Get phase sequence from config
    let phase_sequence = config.phase_sequence();

    // Convert current TaskPhase to phase_id string
    let current_phase_id = state.phase_id();

    // Determine next phase
    let next_phase_id = next_phase(current_phase_id, &phase_sequence);

    match next_phase_id {
        None => {
            // At final phase, no next
            let current_name = current_phase_id.unwrap_or("none");
            Err(WtError::InvalidInput(format!(
                "Task '{}' is in phase '{}' which has no next phase",
                task_name, current_name
            )))
        }
        Some(next_id) => {
            // Stop current process if running
            if state.status == TaskStatus::Active {
                stop_current_process(&config, &state)?;
            }

            // Get phase definition (from config or create default)
            let phase_def = get_phase_definition(&config, next_id);

            // Check if we need to allocate resources
            let needs_resources = phase_def.resources == PhaseResources::Full;
            let has_resources = state.instance.is_some();

            // Allocate resources if needed
            let instance = if needs_resources && !has_resources {
                Some(allocate_resources(&config, &task_name)?)
            } else {
                state.instance.clone()
            };

            // Update status
            let task_state = status_store.get_mut(&task_name);

            // Check if this is the "completed" phase
            if next_id == "completed" {
                task_state.status = TaskStatus::Completed;
                task_state.phase = TaskPhase::None;
                task_state.idle_reason = None;
                task_state.active_since = None;
                task_state.instance = None;

                // Clean up resources if any
                if let Some(ref inst) = instance {
                    let _ = cleanup_resources(&config, inst);
                }

                status_store.save()?;
                println!("Task '{}' marked as completed", task_name);
                return Ok(());
            }

            // Update to new phase
            task_state.phase = phase_id_to_task_phase(next_id);
            task_state.instance = instance.clone();

            // Check if we should execute on_enter workflow
            if let Some(ref workflow) = phase_def.on_enter {
                if !workflow.is_empty() {
                    // Check if first step is an interactive agent
                    let first_step = workflow.steps.first();
                    let is_interactive_agent = first_step.map_or(false, |step| {
                        step.agent.is_some() && step.observe.as_ref().map_or(true, |obs| {
                            obs.mode == ObserveMode::Interactive
                        })
                    });

                    if is_interactive_agent {
                        // Launch interactive agent in multiplexer window
                        if let Some(ref inst) = instance {
                            let agent_step = first_step.unwrap().agent.as_ref().unwrap();
                            start_agent_in_window(&config, &task_name, inst, agent_step, &phase_def.id)?;

                            task_state.status = TaskStatus::Active;
                            task_state.idle_reason = None;
                            task_state.active_since = Some(Utc::now());

                            status_store.save()?;
                            println!(
                                "Task '{}' advanced to phase '{}' (agent started)",
                                task_name, next_id
                            );
                            return Ok(());
                        }
                    }

                    // Execute workflow synchronously (script steps or non-interactive)
                    let result = execute_on_enter(&config, &task_name, &phase_def, instance.as_ref())?;

                    // Update status based on workflow result
                    match result.workflow_state {
                        WorkflowState::Success => {
                            task_state.status = TaskStatus::Idle;
                            task_state.idle_reason = Some(IdleReason::Done);
                            task_state.active_since = None;
                            println!(
                                "Task '{}' advanced to phase '{}' (workflow completed)",
                                task_name, next_id
                            );
                        }
                        WorkflowState::Running => {
                            task_state.status = TaskStatus::Active;
                            task_state.idle_reason = None;
                            task_state.active_since = Some(Utc::now());
                            println!(
                                "Task '{}' advanced to phase '{}' (workflow running)",
                                task_name, next_id
                            );
                        }
                        WorkflowState::Blocked => {
                            task_state.status = TaskStatus::Idle;
                            task_state.idle_reason = Some(IdleReason::HumanReview);
                            task_state.active_since = None;
                            println!(
                                "Task '{}' advanced to phase '{}' (blocked, needs intervention)",
                                task_name, next_id
                            );
                        }
                        WorkflowState::Failed => {
                            task_state.status = TaskStatus::Idle;
                            task_state.idle_reason = Some(IdleReason::Error);
                            task_state.active_since = None;
                            println!(
                                "Task '{}' advanced to phase '{}' (workflow failed)",
                                task_name, next_id
                            );
                        }
                        WorkflowState::Pending => {
                            task_state.status = TaskStatus::Idle;
                            task_state.idle_reason = Some(IdleReason::Done);
                            task_state.active_since = None;
                        }
                    }

                    status_store.save()?;
                    return Ok(());
                }
            }

            // No on_enter workflow - check if we should start default agent
            if let Some(ref inst) = instance {
                // Start default development agent
                let default_agent = crate::models::AgentStep::default_develop(&task_name);
                start_agent_in_window(&config, &task_name, inst, &default_agent, next_id)?;

                task_state.status = TaskStatus::Active;
                task_state.idle_reason = None;
                task_state.active_since = Some(Utc::now());

                status_store.save()?;
                println!("Task '{}' advanced to phase '{}' (agent started)", task_name, next_id);
            } else {
                // No resources, just mark as idle
                task_state.status = TaskStatus::Idle;
                task_state.idle_reason = Some(IdleReason::Done);
                task_state.active_since = None;

                status_store.save()?;
                println!("Task '{}' advanced to phase '{}'", task_name, next_id);
            }

            Ok(())
        }
    }
}

/// Stop the current process in multiplexer window
fn stop_current_process(config: &WtConfig, state: &crate::models::TaskState) -> Result<()> {
    if let Some(instance) = &state.instance {
        let mux = create_multiplexer(config.multiplexer_type());
        // Send Ctrl+C to stop the process
        let _ = mux.send_keys(&instance.session_name, &instance.window_name, "C-c");
        println!("Stopped process in window '{}'", instance.window_name);
    }
    Ok(())
}

/// Get phase definition from config or create default
fn get_phase_definition(config: &WtConfig, phase_id: &str) -> Phase {
    // Try to get from config
    if let Some(phase) = config.get_phase(phase_id) {
        return phase.clone();
    }

    // Create default based on phase name
    match phase_id {
        "pending" | "completed" => Phase::new(phase_id),
        _ => Phase::with_resources(phase_id), // developing, reviewing, etc. need resources
    }
}

/// Convert phase_id string to legacy TaskPhase enum
fn phase_id_to_task_phase(phase_id: &str) -> TaskPhase {
    match phase_id {
        "developing" => TaskPhase::Developing,
        "reviewing" => TaskPhase::Reviewing,
        "merging" => TaskPhase::Merging,
        _ => TaskPhase::Developing, // Default for unknown phases
    }
}

/// Allocate resources for a task (worktree, branch, window)
fn allocate_resources(config: &WtConfig, task_name: &str) -> Result<Instance> {
    let repo_root = git::get_repo_root()?;

    // Generate branch name with timestamp-based suffix
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() % 0xFFFFFF)
        .unwrap_or(0);
    let branch_name = format!("wt/{}-{:06x}", task_name, timestamp);

    // Worktree path
    let worktree_path = format!("{}/{}", config.worktree_dir, task_name);
    let full_worktree_path = if worktree_path.starts_with('/') {
        worktree_path.clone()
    } else {
        format!("{}/{}", repo_root, worktree_path)
    };

    // Create worktree (this also creates the branch)
    git::create_worktree(&branch_name, &full_worktree_path)?;
    println!("Created worktree at {}", full_worktree_path);

    // Create multiplexer window
    let mux = create_multiplexer(config.multiplexer_type());
    let session_name = &config.session_name;
    let window_name = task_name;

    // Ensure session exists
    if !mux.session_exists(session_name) {
        mux.create_session(session_name)?;
    }

    // Create window
    mux.create_window(session_name, window_name, &full_worktree_path, "")?;
    println!("Created window '{}' in session '{}'", window_name, session_name);

    Ok(Instance {
        branch: branch_name,
        worktree_path: full_worktree_path,
        session_name: session_name.clone(),
        window_name: window_name.to_string(),
        session_id: None,
        multiplexer: config.multiplexer_type(),
    })
}

/// Clean up resources (worktree, window)
fn cleanup_resources(config: &WtConfig, instance: &Instance) -> Result<()> {
    // Remove worktree
    let _ = git::remove_worktree(&instance.worktree_path);

    // Close window
    let mux = create_multiplexer(config.multiplexer_type());
    let _ = mux.kill_window(&instance.session_name, &instance.window_name);

    Ok(())
}

/// Start an agent in the multiplexer window
fn start_agent_in_window(
    config: &WtConfig,
    task_name: &str,
    instance: &Instance,
    agent_step: &crate::models::AgentStep,
    phase_id: &str,
) -> Result<()> {
    let repo_root = git::get_repo_root()?;

    // Build execution context
    let context = ExecutionContext::new(
        task_name,
        &instance.branch,
        &instance.worktree_path,
        &repo_root,
    )
    .with_session(&instance.session_name)
    .with_window(&instance.window_name)
    .with_phase(phase_id);

    // Build claude command
    let expanded_prompt = context.expand(&agent_step.prompt);
    let builder = ClaudeCommandBuilder::from_agent_step(agent_step, &context);
    let command = builder.prompt_escaped(&expanded_prompt).build_command_string(&config.claude_command);

    // Send command to multiplexer window
    let mux = create_multiplexer(config.multiplexer_type());

    // Focus the window first
    let _ = mux.focus_window(&instance.session_name, &instance.window_name);

    // Send the command
    mux.send_keys(&instance.session_name, &instance.window_name, &command)?;
    mux.send_keys(&instance.session_name, &instance.window_name, "Enter")?;

    Ok(())
}

/// Execute on_enter workflow
fn execute_on_enter(
    config: &WtConfig,
    task_name: &str,
    phase: &Phase,
    instance: Option<&Instance>,
) -> Result<OnEnterResult> {
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

    // Execute phase transition
    let transition = PhaseTransition::new(config, context)
        .with_log_dir(PathBuf::from(".wt/logs"));

    let _result = transition.enter(phase, &mut runtime_state)?;

    Ok(OnEnterResult {
        workflow_state: runtime_state.workflow_state,
    })
}

/// Result of on_enter workflow execution
struct OnEnterResult {
    workflow_state: WorkflowState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_id_to_task_phase() {
        assert_eq!(phase_id_to_task_phase("developing"), TaskPhase::Developing);
        assert_eq!(phase_id_to_task_phase("reviewing"), TaskPhase::Reviewing);
        assert_eq!(phase_id_to_task_phase("merging"), TaskPhase::Merging);
        assert_eq!(phase_id_to_task_phase("unknown"), TaskPhase::Developing);
    }

    #[test]
    fn test_get_phase_definition_defaults() {
        let config = WtConfig::default();

        let pending = get_phase_definition(&config, "pending");
        assert_eq!(pending.resources, PhaseResources::None);

        let developing = get_phase_definition(&config, "developing");
        assert_eq!(developing.resources, PhaseResources::Full);

        let completed = get_phase_definition(&config, "completed");
        assert_eq!(completed.resources, PhaseResources::None);
    }
}
