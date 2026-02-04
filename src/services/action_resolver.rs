//! Action resolution logic shared between TUI and commands.
//!
//! This module decouples action computation from the TUI layer,
//! allowing the command layer to resolve actions without depending on tui::App.

use crate::models::{Instance, TaskStatus, UserAction, WtConfig};
use crate::services::multiplexer::{create_multiplexer, MultiplexerType};

/// Minimal task info needed for action resolution
pub struct TaskActionContext {
    pub name: String,
    pub status: TaskStatus,
    pub instance: Option<Instance>,
    pub mux_alive: bool,
}

impl TaskActionContext {
    /// Create context from task store data
    pub fn from_store(
        name: &str,
        status: TaskStatus,
        instance: Option<&Instance>,
    ) -> Self {
        let mux_alive = instance
            .and_then(|inst| {
                let window = inst.window_name.as_deref()?;
                let mux = create_multiplexer(inst.multiplexer_type());
                Some(mux.window_exists(&inst.session_name, window))
            })
            .unwrap_or(false);

        Self {
            name: name.to_string(),
            status,
            instance: instance.cloned(),
            mux_alive,
        }
    }
}

/// Check if currently running inside a multiplexer
pub fn is_in_multiplexer(mux_type: MultiplexerType) -> bool {
    match mux_type {
        MultiplexerType::Tmux => std::env::var("TMUX").is_ok(),
        MultiplexerType::Zellij => std::env::var("ZELLIJ").is_ok(),
    }
}

/// Resolve what action to take when user presses Enter on a task.
///
/// - Active + window exists: switch to it
/// - Active + window closed: show resume command
/// - Idle + window exists: switch to it
/// - Idle + window closed: open worktree shell
/// - Outside multiplexer: attach to session
pub fn resolve_enter_action(ctx: &TaskActionContext) -> Option<UserAction> {
    let instance = ctx.instance.as_ref()?;
    let mux_type = instance.multiplexer_type();
    let session = &instance.session_name;
    let window = instance.window_name.as_deref()?;

    let claude_command = WtConfig::load()
        .map(|c| c.claude_command)
        .unwrap_or_else(|_| "claude".to_string());

    if ctx.mux_alive {
        if is_in_multiplexer(mux_type) {
            // Inside multiplexer: switch to target window
            Some(UserAction::SwitchWindow {
                multiplexer: mux_type,
                session: session.clone(),
                window: window.to_string(),
            })
        } else {
            // Outside multiplexer: attach to session
            Some(UserAction::AttachSession {
                multiplexer: mux_type,
                session: session.clone(),
                window: window.to_string(),
            })
        }
    } else {
        // Window closed
        match ctx.status {
            TaskStatus::Idle => {
                // Idle task: open worktree shell
                let worktree_path = instance.worktree_path.clone().unwrap_or_default();
                Some(UserAction::OpenWorktreeShell {
                    multiplexer: mux_type,
                    session: session.clone(),
                    worktree_path,
                    task_name: ctx.name.clone(),
                })
            }
            TaskStatus::Active => {
                // Active task with closed window: show resume command
                let session_id = instance.session_id.as_ref()?;
                let worktree = instance.worktree_path.clone().unwrap_or_default();
                Some(UserAction::ShowResume {
                    worktree,
                    session_id: session_id.clone(),
                    claude_command,
                })
            }
            _ => None,
        }
    }
}

