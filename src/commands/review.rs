use crate::constants::BACKUPS_DIR;
use crate::error::{Result, WtError};
use crate::models::{HookContext, TaskStatus, TaskStore, WtConfig};
use crate::services::{git, hooks::HooksEngine, multiplexer::create_multiplexer};

pub fn execute(task_ref: String) -> Result<()> {
    let config = WtConfig::load()?;
    let mut store = TaskStore::load()?;

    // Resolve task reference (name or index) to actual name
    let name = store.resolve_task_ref(&task_ref)?;

    // Check if scratch environment
    if store.is_scratch(&name) {
        return Err(WtError::InvalidInput(format!(
            "Scratch environment '{}' cannot be marked for review. Use 'wt delete {}' to clean up.",
            name, name
        )));
    }

    // Check task exists and validate transition
    store.ensure_exists(&name)?;
    let prev_status = store.get_status(&name);
    store.validate_transition(&name, TaskStatus::Review)?;

    // Close multiplexer window if still alive
    if let Some(instance) = store.get_instance(&name) {
        let mux = create_multiplexer(instance.multiplexer_type());
        if mux.kill_window_if_exists(&instance.session_name, &instance.window_name)? {
            println!(
                "Closed {} window {}:{}",
                config.multiplexer, instance.session_name, instance.window_name
            );
        }
    }

    // Get repo root and build hook context
    let repo_root = git::get_repo_root()?;
    let hooks = HooksEngine::new(&config);

    let context = if let Some(instance) = store.get_instance(&name) {
        HookContext::new(&name, &instance.branch, &instance.worktree_path, &repo_root)
            .with_session(&instance.session_name)
            .with_window(&instance.window_name)
            .with_status("review")
            .with_prev_status(prev_status.display_name())
            .with_backup_dir(BACKUPS_DIR)
    } else {
        // Minimal context for tasks without instance (shouldn't happen in normal flow)
        HookContext::new(&name, "", "", &repo_root)
            .with_status("review")
            .with_prev_status(prev_status.display_name())
    };

    // Run before_review hook (lint, format checks, etc.)
    hooks.before_review(&context)?;

    // Update status
    store.set_status(&name, TaskStatus::Review);
    store.save_status()?;

    // Run after_review hook
    hooks.after_review(&context)?;

    println!("Task '{}' marked for review.", name);
    println!("To merge into main, run: wt complete {}", name);
    println!("To resume working, run: wt resume {}", name);
    Ok(())
}
