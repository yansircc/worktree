use std::path::Path;

use crate::constants::BACKUPS_DIR;
use crate::error::{Result, WtError};
use crate::models::{TaskStatus, TaskStore, WtConfig};
use crate::services::{git, hooks::HooksEngine, multiplexer::create_multiplexer};

pub fn execute(task_ref: String) -> Result<()> {
    let config = WtConfig::load()?;
    let mut store = TaskStore::load()?;

    // Resolve task reference
    let name = store.resolve_task_ref(&task_ref)?;

    // Check if scratch environment
    if store.is_scratch(&name) {
        return Err(WtError::InvalidInput(format!(
            "Scratch environment '{}' cannot be resumed. Use 'wt run {}' instead.",
            name, name
        )));
    }

    // Check task exists
    store.ensure_exists(&name)?;

    // Verify status is Idle
    let current_status = store.get_status(&name);
    if current_status != TaskStatus::Idle {
        return Err(WtError::InvalidInput(format!(
            "Task '{}' is {} (expected idle). Only idle tasks can be resumed.",
            name,
            current_status.display_name()
        )));
    }

    // Check instance exists
    let instance = store
        .get_instance(&name)
        .ok_or_else(|| WtError::TaskNotStarted(name.clone()))?;

    // Check worktree exists
    let worktree_path = &instance.worktree_path;
    if !Path::new(worktree_path).exists() {
        return Err(WtError::WorktreeNotFound(name.clone()));
    }

    // Get repo root and build hook context
    let repo_root = git::get_repo_root()?;
    let hooks = HooksEngine::new(&config);

    let context = crate::services::hooks::ExecutionContext::new(&name, &instance.branch, &instance.worktree_path, &repo_root)
        .with_session(&instance.session_name)
        .with_window(&instance.window_name)
        .with_status("active")
        .with_prev_status("idle")
        .with_backup_dir(BACKUPS_DIR);

    // Execute "resume" hook
    hooks.execute("resume", &context)?;

    // Restart multiplexer window if closed
    let session_name = &instance.session_name;
    let window_name = &instance.window_name;
    let mux = create_multiplexer(instance.multiplexer_type());

    if !mux.window_exists(session_name, window_name) {
        // Ensure session exists
        if !mux.session_exists(session_name) {
            mux.create_session(session_name)?;
        }

        // Get start_args from config and build command
        let start_args = config.start_args.replace("${task}", &name);
        let claude_cmd = format!("{} {}", config.claude_command, start_args);

        // Create new window
        mux.create_window(session_name, window_name, worktree_path, &claude_cmd)?;
        println!(
            "Restarted {} window {}:{}",
            config.multiplexer, session_name, window_name
        );
    } else {
        println!(
            "{} window {}:{} is still alive",
            config.multiplexer, session_name, window_name
        );
    }

    // Update status to Active
    store.set_status(&name, TaskStatus::Active);
    store.save_status()?;

    println!("Task '{}' resumed.", name);
    Ok(())
}
