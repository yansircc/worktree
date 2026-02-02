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

    /// Start a task (creates worktree and tmux window)
    Start {
        /// Task name to start (required unless --all is used)
        name: Option<String>,

        /// Start all tasks that are ready (no unmerged dependencies)
        #[arg(long)]
        all: bool,
    },

    /// Mark a task as done (ready for review)
    Done {
        /// Task name to mark as done
        name: String,
    },

    /// Mark a task as merged (keeps worktree/branch for review)
    Merged {
        /// Task name to mark as merged
        name: String,
    },

    /// Archive a merged task (cleanup worktree and branch)
    Archive {
        /// Task name to archive
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

    /// Show status of running/done tasks (TUI by default, --json for programmatic use)
    Status {
        /// Output as JSON for programmatic use (non-interactive)
        #[arg(long)]
        json: bool,

        /// Execute action on task (list, done, merged, archive, enter, tail)
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
        /// Optional name (defaults to new-YYYYMMDD-HHMMSS)
        name: Option<String>,
    },

    /// Generate or install shell completions
    Completions {
        #[command(subcommand)]
        action: CompletionsAction,
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
