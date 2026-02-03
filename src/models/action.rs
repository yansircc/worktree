//! User action types shared between TUI and commands.

use crate::services::multiplexer::MultiplexerType;

/// Action to perform (used by TUI and command layer).
/// This enum is shared between the TUI module and the status command,
/// decoupling the command layer from the UI layer.
#[derive(Debug, Clone)]
pub enum UserAction {
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
    /// Open shell in worktree directory (for Idle tasks)
    OpenWorktreeShell {
        multiplexer: MultiplexerType,
        session: String,
        worktree_path: String,
        task_name: String,
    },
}
