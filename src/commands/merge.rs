use std::path::Path;

use crate::error::{Result, WtError};
use crate::models::{TaskStatus, TaskStore, WtConfig};
use crate::services::{git, tmux};

const MERGE_PROMPT_FILE: &str = ".wt/prompts/merge.md";

pub fn execute(task_ref: String, agent_mode: bool) -> Result<()> {
    let config = WtConfig::load()?;
    let store = TaskStore::load()?;

    // Resolve task reference (name or index) to actual name
    let name = store.resolve_task_ref(&task_ref)?;

    // Check if scratch environment
    if store.is_scratch(&name) {
        return Err(WtError::InvalidInput(format!(
            "Scratch environment '{}' cannot be merged. Use 'wt archive {}' to clean up.",
            name, name
        )));
    }

    // Check task exists
    store.ensure_exists(&name)?;

    // Check task status is Done
    let current_status = store.get_status(&name);
    if current_status != TaskStatus::Done {
        return Err(WtError::InvalidInput(format!(
            "Task '{}' is {} (expected done). Run 'wt done {}' first.",
            name,
            current_status.display_name(),
            name
        )));
    }

    // Check instance exists
    let instance = store.get_instance(&name).ok_or_else(|| {
        WtError::TaskNotStarted(name.clone())
    })?;

    // Check worktree exists
    let worktree_path = &instance.worktree_path;
    if !Path::new(worktree_path).exists() {
        return Err(WtError::WorktreeNotFound(name.clone()));
    }

    let branch = &instance.branch;

    // Get main repo root
    let repo_root = git::get_repo_root()?;

    // Close original tmux window if exists
    if let Err(e) = tmux::kill_window_if_exists(&instance.tmux_session, &instance.tmux_window) {
        eprintln!("Warning: Failed to close tmux window: {}", e);
    }

    // Check if prompt file exists
    let prompt_path = Path::new(&repo_root).join(MERGE_PROMPT_FILE);
    if !prompt_path.exists() {
        return Err(WtError::InvalidInput(format!(
            "Merge prompt file not found: {}\nCreate it first or use 'wt status --action merged --task {}' to just mark as merged.",
            MERGE_PROMPT_FILE, name
        )));
    }

    // Build the merge instruction
    let merge_instruction = format!(
        "Merge branch '{}' into main for task '{}'. The worktree is at '{}'.",
        branch, name, worktree_path
    );

    if agent_mode {
        run_claude_agent_mode(&config, &repo_root, &name, &merge_instruction)?;
    } else {
        run_claude_interactive(&config, &repo_root, &name, &merge_instruction)?;
    }

    Ok(())
}

/// Run Claude in interactive TUI mode within a tmux window
fn run_claude_interactive(
    config: &WtConfig,
    repo_root: &str,
    task_name: &str,
    instruction: &str,
) -> Result<()> {
    // Ensure tmux session exists
    if !tmux::session_exists(&config.tmux_session) {
        tmux::create_session(&config.tmux_session)?;
    }

    let window_name = format!("merge-{}", task_name);

    // Build Claude command for interactive mode
    let claude_cmd = format!(
        "{} --system-prompt-file {} '{}'",
        config.claude_command, MERGE_PROMPT_FILE, instruction
    );

    tmux::create_window(&config.tmux_session, &window_name, repo_root, &claude_cmd)?;

    println!("Started merge in tmux window: {}:{}", config.tmux_session, window_name);
    println!("Claude will execute: rebase -> squash merge -> commit -> wt archive");
    println!("\nSwitch to tmux to observe the process.");

    Ok(())
}

/// Run Claude in agent mode (non-interactive, for automation)
fn run_claude_agent_mode(
    config: &WtConfig,
    repo_root: &str,
    task_name: &str,
    instruction: &str,
) -> Result<()> {
    use std::process::Command;

    println!("Running merge in agent mode for task '{}'...", task_name);

    // Build Claude command with agent mode flags
    // Use -p for non-interactive mode with stream-json output
    // Note: prompt must come last after -p flag
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "{} --verbose --output-format=stream-json --system-prompt-file {} --allowedTools 'Bash(*)' 'Read(*)' 'Glob(*)' 'Grep(*)' 'Edit(*)' 'Write(*)' -p '{}'",
            config.claude_command, MERGE_PROMPT_FILE, instruction
        ))
        .current_dir(repo_root)
        .output()
        .map_err(|e| WtError::Script {
            script: "claude".to_string(),
            message: e.to_string(),
        })?;

    if !output.status.success() {
        return Err(WtError::ScriptFailed {
            script: "claude merge".to_string(),
            exit_code: output.status.code(),
        });
    }

    println!("Merge completed successfully.");
    Ok(())
}
