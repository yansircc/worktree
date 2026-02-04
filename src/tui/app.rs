//! Application state for TUI.

use std::time::SystemTime;

use crate::constants::IDLE_THRESHOLD_SECS;
use crate::display::format_duration;
use crate::error::Result;
use crate::models::{TaskStatus, TaskStore, UserAction, WtConfig};
use crate::services::action_resolver::{resolve_enter_action, TaskActionContext};
use crate::services::multiplexer::{create_multiplexer, MultiplexerType};
use crate::services::{git, transcript};

/// A workflow step for display
#[derive(Debug, Clone)]
pub struct StepDisplay {
    pub name: String,
    pub is_agent: bool,
}

/// Task with computed metrics for display
#[derive(Debug, Clone)]
pub struct TaskDisplay {
    pub index: usize,
    pub name: String,
    pub status: TaskStatus,
    pub phase: Option<String>,
    pub duration: Option<String>,
    pub context_percent: u8,
    pub additions: i32,
    pub deletions: i32,
    pub active: bool,
    pub mux_alive: bool,
    pub worktree_path: Option<String>,
    pub multiplexer: Option<MultiplexerType>,
    pub session_name: Option<String>,
    pub window_name: Option<String>,
    pub session_id: Option<String>,
    pub commit_count: i32,
    pub has_conflict: bool,
    pub current_tool: Option<String>,
    pub latest_message: Option<String>,
    pub step_result: Option<String>,
    pub dependencies: Vec<(String, TaskStatus)>,
    pub workflow_steps: Vec<StepDisplay>,
}

/// Application state
pub struct App {
    pub tasks: Vec<TaskDisplay>,
    pub selected: usize,
    pub show_all: bool,
    pub phase_sequence: Vec<String>,
    config: WtConfig,
}

impl App {
    /// Create new app and load initial data
    pub fn new(show_all: bool) -> Result<Self> {
        let config = WtConfig::load()?;
        let phase_sequence = config.phase_sequence().unwrap_or_default();
        let mut app = Self {
            tasks: Vec::new(),
            selected: 0,
            show_all,
            phase_sequence,
            config,
        };
        app.refresh()?;
        Ok(app)
    }

    /// Refresh task data from disk
    pub fn refresh(&mut self) -> Result<()> {
        use std::collections::HashMap;

        let mut store = TaskStore::load()?;
        let mut tasks = Vec::new();
        let mut status_changed = false;

        // Collect task names first to avoid borrow conflict
        let task_names: Vec<String> = store.list().iter().map(|t| t.name().to_string()).collect();

        // Build name -> index mapping (1-based)
        let index_map: HashMap<&str, usize> = task_names
            .iter()
            .enumerate()
            .map(|(i, name)| (name.as_str(), i + 1))
            .collect();

        // Build status map for dependency display
        let status_map: HashMap<String, TaskStatus> = task_names
            .iter()
            .map(|name| (name.clone(), store.status.get_status(name)))
            .collect();

        for task_name in &task_names {
            // Auto-mark as Idle if Active but multiplexer window is closed
            if store.auto_mark_idle_if_needed(task_name)? {
                status_changed = true;
            }

            let status = store.status.get_status(task_name);

            // TUI v2: Show all non-completed tasks (filter completed unless --all)
            if !self.show_all && status == TaskStatus::Completed {
                continue;
            }

            let task = match store.get(task_name) {
                Some(t) => t,
                None => continue, // Skip if task disappeared during iteration
            };
            let instance = store.status.get_instance(task.name());
            let worktree_path = instance.and_then(|i| i.worktree_path.clone());

            // Get phase and step_result from status store
            let phase = store.status.get_phase(task_name).map(|p| p.to_string());
            let step_result = store.status.get_step_result(task_name).map(|r| r.display_name().to_string());

            // Get dependencies with their statuses
            let dependencies: Vec<(String, TaskStatus)> = task
                .depends()
                .iter()
                .map(|dep| {
                    let dep_status = status_map.get(dep).copied().unwrap_or(TaskStatus::Pending);
                    (dep.clone(), dep_status)
                })
                .collect();

            // Multiplexer status
            let mux_alive = if let Some(inst) = instance {
                inst.window_name.as_deref().map_or(false, |window| {
                    let mux = create_multiplexer(inst.multiplexer_type());
                    mux.window_exists(&inst.session_name, window)
                })
            } else {
                false
            };

            let final_status = status;

            // Find transcript path for metrics and latest message
            let transcript_path = instance.and_then(transcript::find_transcript_for_instance);

            // Parse transcript for metrics (duration, context, etc.)
            let transcript_metrics = transcript_path
                .as_ref()
                .and_then(|p| transcript::parse_transcript(p));

            // Get latest message for TUI display
            let latest_message = transcript_path
                .as_ref()
                .and_then(|p| transcript::get_latest_message(p, 50));

            // Duration from transcript timestamps
            let duration = transcript_metrics
                .as_ref()
                .and_then(|m| m.duration_secs())
                .map(format_duration);

            // Git metrics (additions, deletions, commits, conflict)
            let git_metrics = worktree_path.as_deref().and_then(git::get_worktree_metrics);
            let (additions, deletions) = git_metrics
                .as_ref()
                .map(|m| (m.additions, m.deletions))
                .unwrap_or((0, 0));

            // Activity status
            let active = if let Some(ref path) = worktree_path {
                git::get_last_activity(path)
                    .and_then(|last| {
                        SystemTime::now()
                            .duration_since(last)
                            .ok()
                            .map(|d| d.as_secs() < IDLE_THRESHOLD_SECS)
                    })
                    .unwrap_or(false)
            } else {
                false
            };

            // Context from transcript
            let context_percent = transcript_metrics
                .as_ref()
                .map(|m| m.context_percent())
                .unwrap_or(0);

            // Current tool from transcript
            let current_tool = transcript_metrics
                .as_ref()
                .and_then(|m| m.current_tool.clone());

            // Commit count and conflict status from git metrics
            let (commit_count, has_conflict) = git_metrics
                .as_ref()
                .map(|m| (m.commits, m.has_conflict))
                .unwrap_or((0, false));

            // Get multiplexer and session info
            let (multiplexer, session_name, window_name, session_id) = instance
                .map(|i| {
                    (
                        Some(i.multiplexer_type()),
                        Some(i.session_name.clone()),
                        i.window_name.clone(),
                        i.session_id.clone(),
                    )
                })
                .unwrap_or((None, None, None, None));

            // Get workflow steps from phase definition
            let workflow_steps = phase
                .as_ref()
                .and_then(|p| self.config.get_phase(p))
                .and_then(|phase_def| phase_def.on_enter.as_ref())
                .map(|workflow| {
                    workflow
                        .steps
                        .iter()
                        .map(|step| {
                            let name = step
                                .id
                                .clone()
                                .or_else(|| step.name.clone())
                                .unwrap_or_else(|| {
                                    if step.agent.is_some() {
                                        "agent".to_string()
                                    } else {
                                        "script".to_string()
                                    }
                                });
                            StepDisplay {
                                name,
                                is_agent: step.agent.is_some(),
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            tasks.push(TaskDisplay {
                index: *index_map.get(task_name.as_str()).unwrap_or(&0),
                name: task.name().to_string(),
                status: final_status,
                phase,
                duration,
                context_percent,
                additions,
                deletions,
                active,
                mux_alive,
                worktree_path,
                multiplexer,
                session_name,
                window_name,
                session_id,
                commit_count,
                has_conflict,
                current_tool,
                latest_message,
                step_result,
                dependencies,
                workflow_steps,
            });
        }

        // Save status if any task was auto-marked as Done
        if status_changed {
            store.status.save()?;
        }

        self.tasks = tasks;

        // Adjust selection if out of bounds
        if self.selected >= self.tasks.len() && !self.tasks.is_empty() {
            self.selected = self.tasks.len() - 1;
        }

        Ok(())
    }

    /// Get currently selected task
    pub fn selected_task(&self) -> Option<&TaskDisplay> {
        self.tasks.get(self.selected)
    }

    /// Select next task
    pub fn next(&mut self) {
        if !self.tasks.is_empty() {
            self.selected = (self.selected + 1) % self.tasks.len();
        }
    }

    /// Select previous task
    pub fn previous(&mut self) {
        if !self.tasks.is_empty() {
            self.selected = self.selected.checked_sub(1).unwrap_or(self.tasks.len() - 1);
        }
    }

    /// Get action for Enter key on selected task.
    /// Delegates to action_resolver for consistent behavior with command layer.
    pub fn enter_action(&self) -> Option<UserAction> {
        let task = self.selected_task()?;
        let ctx = self.build_action_context(task);
        resolve_enter_action(&ctx)
    }

    /// Build TaskActionContext from TaskDisplay
    fn build_action_context(&self, task: &TaskDisplay) -> TaskActionContext {
        use crate::models::Instance;

        let instance = if let (Some(mux), Some(session), Some(window)) =
            (task.multiplexer, &task.session_name, &task.window_name)
        {
            Some(Instance {
                branch: Some(format!("wt/{}", task.name)),
                worktree_path: task.worktree_path.clone(),
                session_name: session.clone(),
                window_name: Some(window.clone()),
                session_id: task.session_id.clone(),
                multiplexer: mux,
            })
        } else {
            None
        };

        TaskActionContext {
            name: task.name.clone(),
            status: task.status,
            instance,
            mux_alive: task.mux_alive,
        }
    }

    /// Get action to tail selected task's transcript
    pub fn tail_action(&self) -> Option<UserAction> {
        self.selected_task().and_then(|task| {
            // Can tail Active or Idle tasks
            if task.status == TaskStatus::Active || task.status == TaskStatus::Idle {
                Some(UserAction::Tail {
                    name: task.name.clone(),
                })
            } else {
                None
            }
        })
    }
}
