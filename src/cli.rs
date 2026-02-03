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

    /// Complete a task (merge to main and cleanup)
    Complete {
        /// Task name to complete
        name: String,
    },

    /// Delete a task's resources (worktree, branch)
    Delete {
        /// Task name or index
        name: String,

        /// Force delete (skip confirmation for non-completed tasks)
        #[arg(long, short)]
        force: bool,
    },

    /// Advance a task to the next phase
    ///
    /// Forces the task to move to the next phase in the sequence:
    /// pending -> developing -> reviewing -> merging -> completed
    Next {
        /// Task name or index
        task: String,
    },

    /// Reset a task to a specific phase
    ///
    /// By default, resets to pending (cleans up all resources).
    /// Use --to to reset to a different phase (keeps resources).
    Reset {
        /// Task name to reset
        name: String,

        /// Target phase (pending, developing, reviewing, merging)
        /// Default: pending (full cleanup)
        #[arg(long)]
        to: Option<String>,
    },

    /// Show status of active/idle tasks (TUI by default, --json for programmatic use)
    Status {
        /// Output as JSON for programmatic use (non-interactive)
        #[arg(long)]
        json: bool,

        /// Show verbose output (status + phase + idle_reason + active_since)
        #[arg(long, short)]
        verbose: bool,

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

    /// Manually trigger a hook (for debugging)
    Hooks {
        #[command(subcommand)]
        action: HooksAction,
    },

    /// Pause a running task (Active -> Idle)
    Pause {
        /// Task name to pause
        name: String,

        /// Reason for pausing
        #[arg(long, default_value = "manual")]
        reason: String,
    },

    /// Manage background pipelines
    Pipeline {
        #[command(subcommand)]
        action: PipelineAction,
    },

    // ========================================================================
    // Phases v2 Commands
    // ========================================================================

    /// Mark current step status (used by Agent)
    ///
    /// This command is called by the Agent during execution to mark
    /// the current step's completion status.
    Step {
        #[command(subcommand)]
        action: StepAction,
    },

    /// Go back to the previous phase (rollback)
    ///
    /// Stops any running process and moves the task to the previous phase.
    /// Does NOT execute on_enter workflow (rollback mode).
    Prev {
        /// Task name or index
        task: String,
    },

    /// Stop a running task's process
    ///
    /// Sends Ctrl+C to stop the process but keeps the worktree and branch.
    /// Use this to temporarily pause work without losing state.
    Stop {
        /// Task name or index
        task: String,

        /// Also close the multiplexer window
        #[arg(long)]
        kill_window: bool,
    },
}

#[derive(Subcommand)]
pub enum HooksAction {
    /// Run a specific hook manually
    Run {
        /// Hook name (run, review, resume, complete, delete, reset)
        hook: String,

        /// Target task name
        #[arg(long)]
        task: Option<String>,
    },

    /// List available hooks
    List,
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

#[derive(Subcommand)]
pub enum PipelineAction {
    /// List all background pipelines
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show output from a pipeline
    Logs {
        /// Pipeline ID
        id: String,

        /// Follow output (like tail -f)
        #[arg(long, short)]
        follow: bool,

        /// Number of lines to show
        #[arg(short = 'n', default_value = "50")]
        lines: usize,
    },

    /// Kill a running pipeline
    Kill {
        /// Pipeline ID
        id: String,
    },

    /// Clean up old pipeline records
    Cleanup {
        /// Max age in hours (default: 24)
        #[arg(long, default_value = "24")]
        max_age: u64,
    },
}

// ============================================================================
// Phases v2 Subcommands
// ============================================================================

#[derive(Subcommand)]
pub enum StepAction {
    /// Mark current step as completed successfully
    Done,

    /// Mark current step as blocked (needs human intervention)
    Block {
        /// Reason for blocking (optional)
        message: Option<String>,
    },

    /// Mark current step as failed
    Fail {
        /// Reason for failure (optional)
        message: Option<String>,
    },
}
