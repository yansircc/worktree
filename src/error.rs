use thiserror::Error;

#[derive(Error, Debug)]
pub enum WtError {
    #[error("Failed to read config: {0}")]
    ConfigRead(String),

    #[error("Task '{0}' not found")]
    TaskNotFound(String),

    #[error("Invalid task index {index}: valid range is 1-{total}")]
    InvalidTaskIndex { index: usize, total: usize },

    #[error("Task '{0}' already exists")]
    TaskExists(String),

    #[error("Dependency '{0}' not found")]
    DependencyNotFound(String),

    #[error("Git command failed: {0}")]
    Git(String),

    #[error("Tmux command failed: {0}")]
    Tmux(String),

    #[error("Zellij command failed: {0}")]
    Zellij(String),

    #[error("Multiplexer '{0}' is not installed. Please install it first.")]
    MultiplexerNotInstalled(String),

    #[error(
        "Branch '{0}' already exists.\nHint: Run `git branch -D {0}` to delete it, then retry."
    )]
    BranchExists(String),

    #[error("Invalid task file: {0}")]
    InvalidTaskFile(String),

    #[error("Invalid JSON: {0}")]
    InvalidJson(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("IO error during {operation} on '{path}': {message}")]
    Io {
        operation: String,
        path: String,
        message: String,
    },

    #[error("Invalid state transition: cannot change task from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Cannot reset '{task}': task '{dependent}' depends on it and is {status}")]
    HasDependents {
        task: String,
        dependent: String,
        status: String,
    },

    #[error("Task '{0}' has not been started")]
    TaskNotStarted(String),

    #[error("Task '{0}': worktree no longer exists")]
    WorktreeNotFound(String),

    #[error("Task '{0}': transcript not found")]
    TranscriptNotFound(String),

    #[error("Task '{0}': failed to parse transcript")]
    TranscriptParseFailed(String),

    #[error("Task '{0}': no assistant messages found")]
    NoAssistantMessages(String),

    #[error("JSON serialization failed: {0}")]
    JsonSerialize(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, WtError>;
