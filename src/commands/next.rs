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
use crate::models::{StepResult, Instance, TaskStatus, WtConfig};
use crate::services::claude::ClaudeCommandBuilder;
use crate::services::executor::{next_phase, ExecutionContext, PhaseTransition};
use crate::services::git;
use crate::services::multiplexer::create_multiplexer;
use crate::services::resource_manager;
use crate::services::TaskContext;

/// Execute the next command
///
/// # Arguments
/// * `task_ref` - Task name or index
pub fn execute(task_ref: String) -> Result<()> {
    let mut ctx = TaskContext::load(&task_ref)?;
    let task_name = ctx.name().to_string();

    // Get current state (clone for immutable access)
    let state = ctx.store.status.get(&task_name).clone();

    // Check if already completed
    if state.status == TaskStatus::Completed {
        return Err(WtError::InvalidInput(format!(
            "Task '{}' is already completed",
            task_name
        )));
    }

    // Get phase sequence from config
    let phase_sequence = ctx.config.phase_sequence()?;

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
                resource_manager::stop_process(&ctx.config, &state)?;
            }

            // Get phase definition (must be defined in config)
            let phase_def = ctx
                .config
                .get_phase(next_id)
                .ok_or_else(|| {
                    WtError::InvalidInput(format!(
                        "Phase '{}' not defined in config. Run 'wt validate' to check.",
                        next_id
                    ))
                })?
                .clone();

            // Check if this is a terminal phase (task becomes Completed)
            if phase_def.terminal {
                // Clean up resources if any
                if let Some(ref inst) = state.instance {
                    let _ = resource_manager::cleanup_instance(&ctx.config, inst);
                }

                let task_state = ctx.state_mut();
                task_state.status = TaskStatus::Completed;
                task_state.phase = Some(next_id.to_string());
                task_state.step_result = None;
                task_state.active_since = None;
                task_state.instance = None;

                ctx.save_status()?;
                println!("Task '{}' marked as completed", task_name);
                return Ok(());
            }

            // Allocate resources based on phase requirements
            let needs_resources = !phase_def.resources.is_empty();
            let has_resources = state.instance.as_ref().map_or(false, |i| !i.is_empty());

            // Allocate resources if needed
            let instance = if needs_resources && !has_resources {
                Some(resource_manager::allocate_resources(
                    &ctx.config,
                    &task_name,
                    &phase_def.resources,
                )?)
            } else {
                state.instance.clone()
            };

            // Check if we should execute on_enter workflow
            if let Some(ref workflow) = phase_def.on_enter {
                if !workflow.is_empty() {
                    // Find the first interactive agent step
                    let interactive_agent_index = workflow.steps.iter().position(|step| {
                        step.agent.is_some()
                            && step
                                .observe
                                .as_ref()
                                .map_or(true, |obs| obs.mode == ObserveMode::Interactive)
                    });

                    if let Some(agent_idx) = interactive_agent_index {
                        // Execute script steps before the agent synchronously
                        if agent_idx > 0 {
                            let result = execute_on_enter_until(
                                &ctx.config,
                                &task_name,
                                &phase_def,
                                instance.as_ref(),
                                agent_idx,
                            )?;

                            // If any script step failed, don't start the agent
                            if result.workflow_state == WorkflowState::Failed {
                                let task_state = ctx.state_mut();
                                task_state.phase = Some(next_id.to_string());
                                task_state.instance = instance.clone();
                                task_state.status = TaskStatus::Idle;
                                task_state.step_result = Some(StepResult::Error);
                                task_state.active_since = None;
                                ctx.save_status()?;
                                println!(
                                    "Task '{}' advanced to phase '{}' (workflow failed)",
                                    task_name, next_id
                                );
                                return Ok(());
                            }
                        }

                        // Launch interactive agent in multiplexer window
                        if let Some(ref inst) = instance {
                            let agent_step = workflow.steps[agent_idx].agent.as_ref().unwrap();
                            start_agent_in_window(
                                &ctx.config,
                                &task_name,
                                inst,
                                agent_step,
                                &phase_def.id,
                                Some(agent_idx),
                            )?;

                            // Update state
                            let task_state = ctx.state_mut();
                            task_state.phase = Some(next_id.to_string());
                            task_state.instance = instance.clone();
                            task_state.status = TaskStatus::Active;
                            task_state.step_result = None;
                            task_state.active_since = Some(Utc::now());

                            ctx.save_status()?;
                            println!(
                                "Task '{}' advanced to phase '{}' (agent started)",
                                task_name, next_id
                            );
                            return Ok(());
                        }
                    }

                    // No interactive agent - execute workflow synchronously
                    let result =
                        execute_on_enter(&ctx.config, &task_name, &phase_def, instance.as_ref())?;

                    // Update state based on workflow result
                    let task_state = ctx.state_mut();
                    task_state.phase = Some(next_id.to_string());
                    task_state.instance = instance.clone();

                    match result.workflow_state {
                        WorkflowState::Success => {
                            task_state.status = TaskStatus::Idle;
                            task_state.step_result = Some(StepResult::Done);
                            task_state.active_since = None;
                            ctx.save_status()?;
                            println!(
                                "Task '{}' advanced to phase '{}' (workflow completed)",
                                task_name, next_id
                            );
                        }
                        WorkflowState::Running => {
                            task_state.status = TaskStatus::Active;
                            task_state.step_result = None;
                            task_state.active_since = Some(Utc::now());
                            ctx.save_status()?;
                            println!(
                                "Task '{}' advanced to phase '{}' (workflow running)",
                                task_name, next_id
                            );
                        }
                        WorkflowState::Blocked => {
                            task_state.status = TaskStatus::Idle;
                            task_state.step_result = Some(StepResult::HumanReview);
                            task_state.active_since = None;
                            ctx.save_status()?;
                            println!(
                                "Task '{}' advanced to phase '{}' (blocked, needs intervention)",
                                task_name, next_id
                            );
                        }
                        WorkflowState::Failed => {
                            task_state.status = TaskStatus::Idle;
                            task_state.step_result = Some(StepResult::Error);
                            task_state.active_since = None;
                            ctx.save_status()?;
                            println!(
                                "Task '{}' advanced to phase '{}' (workflow failed)",
                                task_name, next_id
                            );
                        }
                        WorkflowState::Pending => {
                            task_state.status = TaskStatus::Idle;
                            task_state.step_result = Some(StepResult::Done);
                            task_state.active_since = None;
                            ctx.save_status()?;
                        }
                    }

                    return Ok(());
                }
            }

            // No on_enter workflow - check if we should start default agent
            if let Some(ref inst) = instance {
                // Start default development agent
                let default_agent = crate::models::AgentStep::default_develop(&task_name);
                start_agent_in_window(&ctx.config, &task_name, inst, &default_agent, next_id, None)?;

                let task_state = ctx.state_mut();
                task_state.phase = Some(next_id.to_string());
                task_state.instance = instance.clone();
                task_state.status = TaskStatus::Active;
                task_state.step_result = None;
                task_state.active_since = Some(Utc::now());

                ctx.save_status()?;
                println!(
                    "Task '{}' advanced to phase '{}' (agent started)",
                    task_name, next_id
                );
            } else {
                // No resources, just mark as idle
                let task_state = ctx.state_mut();
                task_state.phase = Some(next_id.to_string());
                task_state.instance = instance.clone();
                task_state.status = TaskStatus::Idle;
                task_state.step_result = Some(StepResult::Done);
                task_state.active_since = None;

                ctx.save_status()?;
                println!("Task '{}' advanced to phase '{}'", task_name, next_id);
            }

            Ok(())
        }
    }
}

/// Start an agent in the multiplexer window
fn start_agent_in_window(
    config: &WtConfig,
    task_name: &str,
    instance: &Instance,
    agent_step: &crate::models::AgentStep,
    phase_id: &str,
    step_index: Option<usize>,
) -> Result<()> {
    let repo_root = git::get_repo_root()?;
    let default_branch = format!("wt/{}", task_name);
    let branch = instance.branch.as_deref().unwrap_or(&default_branch);
    let worktree = instance.worktree_path.as_deref().unwrap_or(&repo_root);
    let window = instance.window_name.as_deref().unwrap_or(task_name);

    // Build execution context
    let context = ExecutionContext::new(task_name, branch, worktree, &repo_root)
        .with_session(&instance.session_name)
        .with_window(window)
        .with_phase(phase_id);

    // Build claude command
    let expanded_prompt = context.expand(&agent_step.prompt);
    let builder = ClaudeCommandBuilder::from_agent_step(agent_step, &context);
    let claude_command = builder
        .prompt_escaped(&expanded_prompt)
        .build_command_string(&config.claude_command);

    // Wrap command with environment variables for wt step command
    let step_env = step_index
        .map(|i| format!("WT_STEP={} ", i))
        .unwrap_or_default();
    let command = format!("WT_TASK={} WT_PHASE={} {}{}", task_name, phase_id, step_env, claude_command);

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
    execute_on_enter_until(config, task_name, phase, instance, usize::MAX)
}

/// Execute on_enter workflow until (but not including) the specified step index
fn execute_on_enter_until(
    config: &WtConfig,
    task_name: &str,
    phase: &Phase,
    instance: Option<&Instance>,
    until_step: usize,
) -> Result<OnEnterResult> {
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

    // If we need to execute only a subset of steps, create a modified phase
    let (phase_to_execute, total_steps) = if until_step < usize::MAX {
        if let Some(ref workflow) = phase.on_enter {
            let total = workflow.steps.len();
            let mut modified_phase = phase.clone();
            let mut modified_workflow = workflow.clone();
            modified_workflow.steps = workflow.steps.iter().take(until_step).cloned().collect();
            modified_phase.on_enter = Some(modified_workflow);
            (modified_phase, Some(total))
        } else {
            (phase.clone(), None)
        }
    } else {
        (phase.clone(), None)
    };

    // Execute phase transition with progress output
    let mut transition = PhaseTransition::new(config, context)
        .with_log_dir(PathBuf::from(".wt/logs"))
        .with_progress(true);

    if let Some(total) = total_steps {
        transition = transition.with_total_steps(total);
    }

    let _result = transition.enter(&phase_to_execute, &mut runtime_state)?;

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
        definitions.insert(
            "developing".to_string(),
            Phase::with_resources("developing"),
        );
        let mut completed = Phase::new("completed");
        completed.terminal = true;
        definitions.insert("completed".to_string(), completed);

        let mut config = WtConfig::default();
        config.phases = Some(PhasesConfig {
            sequence: vec![
                "pending".to_string(),
                "developing".to_string(),
                "completed".to_string(),
            ],
            definitions,
        });

        let pending = config.get_phase("pending").unwrap();
        assert!(pending.resources.is_empty());
        assert!(!pending.terminal);

        let developing = config.get_phase("developing").unwrap();
        assert_eq!(
            developing.resources,
            crate::models::phase::PhaseResources::full()
        );
        assert!(!developing.terminal);

        let completed = config.get_phase("completed").unwrap();
        assert!(completed.resources.is_empty());
        assert!(completed.terminal);
    }
}
