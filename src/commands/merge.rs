use std::path::Path;

use crate::error::{Result, WtError};
use crate::models::{TaskStatus, TaskStore, WtConfig};
use crate::services::git;
use crate::services::multiplexer::create_multiplexer;

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

    // Close original multiplexer window if exists
    let mux = create_multiplexer(instance.multiplexer_type());
    if let Err(e) = mux.kill_window_if_exists(&instance.session_name, &instance.window_name) {
        eprintln!("Warning: Failed to close {} window: {}", instance.multiplexer, e);
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

/// Escape a string for safe use in shell commands (single-quoted)
fn shell_escape(s: &str) -> String {
    // Replace single quotes with '\'' (end quote, escaped quote, start quote)
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Run Claude in interactive TUI mode within a multiplexer window
fn run_claude_interactive(
    config: &WtConfig,
    repo_root: &str,
    task_name: &str,
    instruction: &str,
) -> Result<()> {
    let mux = config.create_multiplexer();
    let session = &config.session_name;

    // Ensure session exists
    if !mux.session_exists(session) {
        mux.create_session(session)?;
    }

    let window_name = format!("merge-{}", task_name);

    // Build Claude command for interactive mode with proper escaping
    let claude_cmd = format!(
        "{} --system-prompt-file {} {}",
        config.claude_command, MERGE_PROMPT_FILE, shell_escape(instruction)
    );

    mux.create_window(session, &window_name, repo_root, &claude_cmd)?;

    println!("Started merge in {} window: {}:{}", config.multiplexer_type(), session, window_name);
    println!("Claude will execute: rebase -> squash merge -> commit -> wt archive");
    println!("\nSwitch to {} to observe the process.", config.multiplexer_type());

    Ok(())
}

/// Run Claude in agent mode (non-interactive, for automation)
fn run_claude_agent_mode(
    config: &WtConfig,
    repo_root: &str,
    task_name: &str,
    instruction: &str,
) -> Result<()> {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    println!("Running merge in agent mode for task '{}'...", task_name);

    // Build Claude command with agent mode flags and proper escaping
    // Use -p for non-interactive mode with stream-json output
    let cmd_str = format!(
        "{} --verbose --output-format=stream-json --system-prompt-file {} --allowedTools 'Bash(*)' 'Read(*)' 'Glob(*)' 'Grep(*)' 'Edit(*)' 'Write(*)' -p {}",
        config.claude_command, MERGE_PROMPT_FILE, shell_escape(instruction)
    );

    let mut child = Command::new("sh")
        .arg("-c")
        .arg(&cmd_str)
        .current_dir(repo_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| WtError::Script {
            script: "claude".to_string(),
            message: e.to_string(),
        })?;

    // Stream stdout in real-time
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                println!("{}", line);
            }
        }
    }

    let status = child.wait().map_err(|e| WtError::Script {
        script: "claude".to_string(),
        message: e.to_string(),
    })?;

    if !status.success() {
        return Err(WtError::ScriptFailed {
            script: "claude merge".to_string(),
            exit_code: status.code(),
        });
    }

    println!("Merge completed successfully.");
    Ok(())
}
