//! Application state for TUI.

use std::time::SystemTime;

use crate::constants::IDLE_THRESHOLD_SECS;
use crate::display::format_duration;
use crate::error::Result;
use crate::models::{TaskStatus, TaskStore, WtConfig};
use crate::services::{git, multiplexer::MultiplexerType, transcript};
use crate::services::multiplexer::create_multiplexer;

/// Action to perform after TUI exits or during TUI
#[derive(Debug, Clone)]
pub enum TuiAction {
    /// Just quit, no action
    Quit,
    /// Switch to multiplexer window (inside multiplexer, window exists)
    SwitchWindow {
        multiplexer: MultiplexerType,
        session: String,
        window: String,
    },
    /// Attach to multiplexer session (outside multiplexer, window exists)
    AttachSession {
        multiplexer: MultiplexerType,
        session: String,
        window: String,
    },
    /// Show resume command (multiplexer window closed, need to copy command)
    ShowResume {
        worktree: String,
        session_id: String,
        claude_command: String,
    },
    /// Tail a task's transcript
    Tail { name: String },
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
    pub idle_reason: Option<String>,
    pub dependencies: Vec<(String, TaskStatus)>,
}

/// Application state
pub struct App {
    pub tasks: Vec<TaskDisplay>,
    pub selected: usize,
    pub show_all: bool,
}

impl App {
    /// Create new app and load initial data
    pub fn new(show_all: bool) -> Result<Self> {
        let mut app = Self {
            tasks: Vec::new(),
            selected: 0,
            show_all,
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
            .map(|name| (name.clone(), store.get_status(name)))
            .collect();

        for task_name in &task_names {
            // Auto-mark as Idle if Active but multiplexer window is closed
            if store.auto_mark_idle_if_needed(task_name)? {
                status_changed = true;
            }

            let status = store.get_status(task_name);

            // TUI v2: Show all non-completed tasks (filter completed unless --all)
            if !self.show_all && status == TaskStatus::Completed {
                continue;
            }

            let task = store.get(task_name).unwrap();
            let instance = store.get_instance(task.name());
            let worktree_path = instance.map(|i| i.worktree_path.clone());

            // Get phase and idle_reason from status store
            let phase = store.get_phase(task_name).map(|p| p.display_name().to_string());
            let idle_reason = store.get_idle_reason(task_name).map(|r| r.display_name().to_string());

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
                let mux = create_multiplexer(inst.multiplexer_type());
                mux.window_exists(&inst.session_name, &inst.window_name)
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
                        Some(i.window_name.clone()),
                        i.session_id.clone(),
                    )
                })
                .unwrap_or((None, None, None, None));

            tasks.push(TaskDisplay {
                index: index_map[task_name.as_str()],
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
                idle_reason,
                dependencies,
            });
        }

        // Save status if any task was auto-marked as Done
        if status_changed {
            store.save_status()?;
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

    /// Check if running inside a multiplexer
    pub fn is_in_multiplexer(&self, mux_type: MultiplexerType) -> bool {
        match mux_type {
            MultiplexerType::Tmux => std::env::var("TMUX").is_ok(),
            MultiplexerType::Zellij => std::env::var("ZELLIJ").is_ok(),
        }
    }

    /// Get action for Enter key on selected task
    /// - Inside multiplexer + window exists: switch to it
    /// - Inside multiplexer + window closed: show resume command
    /// - Outside multiplexer: show attach command
    pub fn enter_action(&self) -> Option<TuiAction> {
        let task = self.selected_task()?;

        // Need multiplexer and session info
        let mux_type = task.multiplexer?;
        let session = task.session_name.as_ref()?;
        let window = task.window_name.as_ref()?;

        let claude_command = WtConfig::load()
            .map(|c| c.claude_command)
            .unwrap_or_else(|_| "claude".to_string());

        if task.mux_alive {
            if self.is_in_multiplexer(mux_type) {
                // Inside multiplexer: switch to target window
                Some(TuiAction::SwitchWindow {
                    multiplexer: mux_type,
                    session: session.clone(),
                    window: window.clone(),
                })
            } else {
                // Outside multiplexer: attach to session
                Some(TuiAction::AttachSession {
                    multiplexer: mux_type,
                    session: session.clone(),
                    window: window.clone(),
                })
            }
        } else {
            // Multiplexer window closed, show resume command
            let worktree = task.worktree_path.as_ref()?;
            let session_id = task.session_id.as_ref()?;
            Some(TuiAction::ShowResume {
                worktree: worktree.clone(),
                session_id: session_id.clone(),
                claude_command,
            })
        }
    }

    /// Get action to tail selected task's transcript
    pub fn tail_action(&self) -> Option<TuiAction> {
        self.selected_task().and_then(|task| {
            // Can tail Active or Idle tasks
            if task.status == TaskStatus::Active || task.status == TaskStatus::Idle {
                Some(TuiAction::Tail {
                    name: task.name.clone(),
                })
            } else {
                None
            }
        })
    }
}
