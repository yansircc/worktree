//! Complete command - merge a task's changes to main and clean up.

use std::path::Path;

use chrono::Utc;

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
            "Scratch environment '{}' cannot be completed. Use 'wt delete {}' to clean up.",
            name, name
        )));
    }

    // Check task exists
    store.ensure_exists(&name)?;

    // Check task status is Idle
    let current_status = store.get_status(&name);
    if current_status != TaskStatus::Idle {
        return Err(WtError::InvalidInput(format!(
            "Task '{}' is {} (expected idle). Run 'wt review {}' first.",
            name,
            current_status.display_name(),
            name
        )));
    }

    // Check instance exists
    let instance = store
        .get_instance(&name)
        .ok_or_else(|| WtError::TaskNotStarted(name.clone()))?
        .clone();

    // Check worktree exists
    let worktree_path = &instance.worktree_path;
    if !Path::new(worktree_path).exists() {
        return Err(WtError::WorktreeNotFound(name.clone()));
    }

    // Get main repo root
    let repo_root = git::get_repo_root()?;

    // Build hook context
    let context = crate::services::hooks::ExecutionContext::new(&name, &instance.branch, worktree_path, &repo_root)
        .with_session(&instance.session_name)
        .with_window(&instance.window_name)
        .with_status("completed")
        .with_prev_status("idle")
        .with_timestamp(&Utc::now().to_rfc3339());

    // Create hooks engine
    let hooks = HooksEngine::new(&config);

    // Execute "complete" hook
    println!("Running complete hook...");
    hooks.execute("complete", &context)?;

    // Execute default merge flow if no custom hook handles it
    if !config.has_custom_complete_hook() {
        default_merge_flow(&repo_root, worktree_path, &instance.branch, &name)?;
    }

    // Clean up resources
    cleanup_resources(&config, &instance, &repo_root)?;

    // Update status
    store.set_status(&name, TaskStatus::Completed);
    store.save_status()?;

    println!("Task '{}' completed and merged to main.", name);
    Ok(())
}

/// Default merge flow: rebase onto main, squash merge, commit
fn default_merge_flow(
    repo_root: &str,
    worktree_path: &str,
    branch: &str,
    task_name: &str,
) -> Result<()> {
    // 1. Fetch latest main
    println!("Fetching latest changes...");
    if let Err(e) = git::fetch(repo_root, "origin") {
        eprintln!("  Warning: Failed to fetch: {}", e);
    }

    // 2. Rebase onto origin/main
    println!("Rebasing onto main...");
    match git::rebase(worktree_path, "origin/main") {
        Ok(git::RebaseResult::Success) => {
            println!("  Rebase successful.");
        }
        Ok(git::RebaseResult::AlreadyUpToDate) => {
            println!("  Already up to date.");
        }
        Ok(git::RebaseResult::Conflicts) => {
            return Err(WtError::Git(
                "Rebase conflicts detected. Please resolve conflicts manually and try again."
                    .to_string(),
            ));
        }
        Err(e) => {
            return Err(e);
        }
    }

    // 3. Checkout main in repo root
    println!("Checking out main...");
    git::checkout(repo_root, "main")?;

    // 4. Squash merge
    println!("Squash merging {}...", branch);
    git::squash_merge(repo_root, branch)?;

    // 5. Commit with a descriptive message
    let message = format!(
        "feat({}): complete task\n\nSquash merge from branch '{}'",
        task_name, branch
    );
    println!("Creating commit...");
    git::commit(repo_root, &message)?;

    Ok(())
}

/// Clean up resources after completion
fn cleanup_resources(
    config: &WtConfig,
    instance: &crate::models::Instance,
    repo_root: &str,
) -> Result<()> {
    println!("Cleaning up resources...");

    // Kill multiplexer window if exists
    let mux = create_multiplexer(instance.multiplexer_type());
    if let Err(e) = mux.kill_window_if_exists(&instance.session_name, &instance.window_name) {
        eprintln!(
            "  Warning: Failed to close {} window: {}",
            config.multiplexer, e
        );
    } else {
        println!(
            "  Closed {} window: {}:{}",
            config.multiplexer, instance.session_name, instance.window_name
        );
    }

    // Remove worktree
    if let Err(e) = git::remove_worktree(&instance.worktree_path) {
        eprintln!("  Warning: Failed to remove worktree: {}", e);
    } else {
        println!("  Removed worktree: {}", instance.worktree_path);
    }

    // Delete branch
    if let Err(e) = git::delete_branch(repo_root, &instance.branch) {
        eprintln!("  Warning: Failed to delete branch: {}", e);
    } else {
        println!("  Deleted branch: {}", instance.branch);
    }

    Ok(())
}
