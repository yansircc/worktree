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
use crate::models::phase::Phase;
use crate::models::state::TaskRuntimeState;
use crate::models::step::ObserveMode;
use crate::models::workflow::WorkflowState;
use crate::models::{IdleReason, Instance, StatusStore, TaskStatus, TaskStore, WtConfig};
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
    let phase_sequence = config.phase_sequence()?;

    // Get current phase_id from state
    let current_phase_id = state.phase.as_deref();

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

            // Get phase definition (must be defined in config)
            let phase_def = config.get_phase(next_id)
                .ok_or_else(|| WtError::InvalidInput(format!(
                    "Phase '{}' not defined in config. Run 'wt validate' to check.",
                    next_id
                )))?
                .clone();

            // Check if this is a terminal phase (task becomes Completed)
            if phase_def.terminal {
                let task_state = status_store.get_mut(&task_name);
                task_state.status = TaskStatus::Completed;
                task_state.phase = Some(next_id.to_string());
                task_state.idle_reason = None;
                task_state.active_since = None;
                task_state.instance = None;

                // Clean up resources if any
                if let Some(ref inst) = state.instance {
                    let _ = cleanup_resources(&config, inst);
                }

                status_store.save()?;
                println!("Task '{}' marked as completed", task_name);
                return Ok(());
            }

            // Allocate resources based on phase requirements
            let needs_resources = !phase_def.resources.is_empty();
            let has_resources = state.instance.as_ref().map_or(false, |i| !i.is_empty());

            // Allocate resources if needed
            let instance = if needs_resources && !has_resources {
                Some(allocate_resources(&config, &task_name, &phase_def.resources)?)
            } else {
                state.instance.clone()
            };

            // Update status
            let task_state = status_store.get_mut(&task_name);

            // Update to new phase
            task_state.phase = Some(next_id.to_string());
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
        if let Some(ref window) = instance.window_name {
            let mux = create_multiplexer(config.multiplexer_type());
            // Send Ctrl+C to stop the process
            let _ = mux.send_keys(&instance.session_name, window, "C-c");
            println!("Stopped process in window '{}'", window);
        }
    }
    Ok(())
}

/// Allocate resources for a task based on phase resource requirements
fn allocate_resources(
    config: &WtConfig,
    task_name: &str,
    resources: &crate::models::phase::PhaseResources,
) -> Result<Instance> {
    let repo_root = git::get_repo_root()?;

    let mut instance = Instance {
        branch: None,
        worktree_path: None,
        session_name: config.session_name.clone(),
        window_name: None,
        session_id: None,
        multiplexer: config.multiplexer_type(),
    };

    // Create branch if needed
    if resources.branch {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() % 0xFFFFFF)
            .unwrap_or(0);
        let branch_name = format!("wt/{}-{:06x}", task_name, timestamp);
        instance.branch = Some(branch_name);
    }

    // Create worktree if needed (requires branch)
    if resources.worktree {
        let branch_name = instance.branch.as_ref()
            .ok_or_else(|| WtError::InvalidInput("worktree requires branch".into()))?;

        let worktree_path = format!("{}/{}", config.worktree_dir, task_name);
        let full_worktree_path = if worktree_path.starts_with('/') {
            worktree_path.clone()
        } else {
            format!("{}/{}", repo_root, worktree_path)
        };

        git::create_worktree(branch_name, &full_worktree_path)?;
        println!("Created worktree at {}", full_worktree_path);
        instance.worktree_path = Some(full_worktree_path);
    }

    // Create multiplexer window if needed
    if resources.window {
        let cwd = instance.worktree_path.as_deref().unwrap_or(".");
        let mux = create_multiplexer(config.multiplexer_type());
        let session_name = &config.session_name;

        if !mux.session_exists(session_name) {
            mux.create_session(session_name)?;
        }

        mux.create_window(session_name, task_name, cwd, "")?;
        println!("Created window '{}' in session '{}'", task_name, session_name);
        instance.window_name = Some(task_name.to_string());
    }

    Ok(instance)
}

/// Clean up resources (window → worktree → branch)
fn cleanup_resources(config: &WtConfig, instance: &Instance) -> Result<()> {
    // Close window
    if let Some(ref window) = instance.window_name {
        let mux = create_multiplexer(config.multiplexer_type());
        let _ = mux.kill_window(&instance.session_name, window);
    }

    // Remove worktree
    if let Some(ref path) = instance.worktree_path {
        let _ = git::remove_worktree(path);
    }

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
    let default_branch = format!("wt/{}", task_name);
    let branch = instance.branch.as_deref().unwrap_or(&default_branch);
    let worktree = instance.worktree_path.as_deref().unwrap_or(&repo_root);
    let window = instance.window_name.as_deref().unwrap_or(task_name);

    // Build execution context
    let context = ExecutionContext::new(
        task_name,
        branch,
        worktree,
        &repo_root,
    )
    .with_session(&instance.session_name)
    .with_window(window)
    .with_phase(phase_id);

    // Build claude command
    let expanded_prompt = context.expand(&agent_step.prompt);
    let builder = ClaudeCommandBuilder::from_agent_step(agent_step, &context);
    let command = builder.prompt_escaped(&expanded_prompt).build_command_string(&config.claude_command);

    // Send command to multiplexer window
    let mux = create_multiplexer(config.multiplexer_type());

    // Focus the window first
    let _ = mux.focus_window(&instance.session_name, window);

    // Send the command
    mux.send_keys(&instance.session_name, window, &command)?;
    mux.send_keys(&instance.session_name, window, "Enter")?;

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
    let default_branch = format!("wt/{}", task_name);
    let branch = instance.and_then(|i| i.branch.as_deref()).unwrap_or(&default_branch);
    let worktree = instance.and_then(|i| i.worktree_path.as_deref()).unwrap_or(&repo_root);
    let session = instance.map(|i| i.session_name.as_str()).unwrap_or(&config.session_name);
    let window = instance.and_then(|i| i.window_name.as_deref()).unwrap_or(task_name);

    let context = ExecutionContext::new(task_name, branch, worktree, &repo_root)
        .with_session(session)
        .with_window(window)
        .with_phase(&phase.id);

    // Create runtime state
    let mut runtime_state = TaskRuntimeState::pending();

    // Execute phase transition with progress output
    let transition = PhaseTransition::new(config, context)
        .with_log_dir(PathBuf::from(".wt/logs"))
        .with_progress(true);

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
    fn test_phase_terminal_from_config() {
        use crate::models::phase::Phase;
        use crate::models::project::PhasesConfig;
        use std::collections::HashMap;

        let mut definitions = HashMap::new();
        definitions.insert("pending".to_string(), Phase::new("pending"));
        definitions.insert("developing".to_string(), Phase::with_resources("developing"));
        let mut completed = Phase::new("completed");
        completed.terminal = true;
        definitions.insert("completed".to_string(), completed);

        let mut config = WtConfig::default();
        config.phases = Some(PhasesConfig {
            sequence: vec!["pending".to_string(), "developing".to_string(), "completed".to_string()],
            definitions,
        });

        let pending = config.get_phase("pending").unwrap();
        assert!(pending.resources.is_empty());
        assert!(!pending.terminal);

        let developing = config.get_phase("developing").unwrap();
        assert_eq!(developing.resources, crate::models::phase::PhaseResources::full());
        assert!(!developing.terminal);

        let completed = config.get_phase("completed").unwrap();
        assert!(completed.resources.is_empty());
        assert!(completed.terminal);
    }
}
