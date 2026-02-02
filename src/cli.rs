use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser)]
#[command(name = "wt")]
#[command(about = "Worktree Task Manager - manage multi-agent parallel development tasks")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize wt in current directory
    Init,

    /// Create a new task from JSON
    Create {
        /// JSON input: {"name": "...", "depends": [...], "description": "..."}
        #[arg(long)]
        json: String,
    },

    /// Validate all task files
    Validate {
        /// Specific task name to validate (optional)
        name: Option<String>,
    },

    /// List all tasks (grouped by status)
    List {
        /// Show tree view instead of grouped view
        #[arg(long)]
        tree: bool,

        /// Output as JSON for programmatic use
        #[arg(long)]
        json: bool,
    },

    /// Run a task (creates worktree if needed, starts Claude)
    Run {
        /// Task name or index (required unless --all is used)
        task: Option<String>,

        /// Start all tasks that are ready (no unmerged dependencies)
        #[arg(long)]
        all: bool,
    },

    /// Start a task (alias for 'run', kept for backward compatibility)
    #[command(hide = true)]
    Start {
        /// Task name to start (required unless --all is used)
        name: Option<String>,

        /// Start all tasks that are ready (no unmerged dependencies)
        #[arg(long)]
        all: bool,
    },

    /// Mark a task for review (ready to be merged)
    Review {
        /// Task name to mark for review
        name: String,
    },

    /// Resume a task from review state back to running
    Resume {
        /// Task name to resume
        name: String,
    },

    /// Execute merge via Claude (rebase + squash merge + cleanup)
    Merge {
        /// Task name to merge
        name: String,

        /// Run in agent mode (non-interactive, for automation)
        #[arg(long)]
        agent: bool,
    },

    /// Delete a scratch environment
    Delete {
        /// Scratch environment name to delete
        name: String,
    },

    /// Show tasks that are ready to start (all dependencies merged)
    Next {
        /// Output as JSON for programmatic use
        #[arg(long)]
        json: bool,
    },

    /// Reset a task to pending state (cleanup resources)
    Reset {
        /// Task name to reset
        name: String,
    },

    /// Show status of running/review tasks (TUI by default, --json for programmatic use)
    Status {
        /// Output as JSON for programmatic use (non-interactive)
        #[arg(long)]
        json: bool,

        /// Execute action on task (list, review, resume, complete, enter, tail)
        #[arg(long, value_name = "ACTION")]
        action: Option<String>,

        /// Target task name (required with --action)
        #[arg(long, value_name = "NAME")]
        task: Option<String>,
    },

    /// View last assistant messages from task transcript (JSON output)
    Tail {
        /// Task name
        name: String,

        /// Number of turns to show (default: 1)
        #[arg(short = 'n', default_value = "1")]
        count: usize,
    },

    /// Generate filtered logs for all tasks
    Logs,

    /// Create a scratch environment (quick worktree without task file)
    New {
        /// Optional name (defaults to s1, s2, ...)
        name: Option<String>,

        /// Only print the worktree path (for shell integration)
        #[arg(long)]
        print_path: bool,
    },

    /// Generate or install shell completions
    Completions {
        #[command(subcommand)]
        action: CompletionsAction,
    },

    /// Internal commands for hooks (not for direct user use)
    #[command(hide = true)]
    Internal {
        /// Operation in format "category:action" (e.g., "mux:focus-window", "git:fetch")
        operation: String,

        /// Arguments for the operation
        args: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum CompletionsAction {
    /// Generate completions script to stdout
    Generate {
        /// Shell to generate completions for
        shell: Shell,
    },
    /// Install completions to shell config (auto-detects shell)
    Install,
}
