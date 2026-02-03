//! Complete command - merge a task's changes to main and clean up.

use chrono::Utc;

use crate::error::{Result, WtError};
use crate::models::{Instance, TaskStatus, WtConfig};
use crate::services::{git, hooks::HooksEngine, multiplexer::create_multiplexer, TaskContext};

pub fn execute(task_ref: String) -> Result<()> {
    let mut ctx = TaskContext::load(&task_ref)?;

    // Validate (check scratch first to give better error message)
    ctx.require_not_scratch("completed")?;
    ctx.store.ensure_exists(ctx.name())?;
    ctx.require_status(&[TaskStatus::Idle], "complete")?;
    ctx.require_worktree()?;

    let instance = ctx.require_instance()?.clone();
    let repo_root = ctx.repo_root()?.to_string();

    // Build hook context
    let hook_ctx = ctx
        .build_hook_context()?
        .with_status("completed")
        .with_prev_status("idle")
        .with_timestamp(&Utc::now().to_rfc3339());

    // Execute "complete" hook
    let hooks = HooksEngine::new(&ctx.config);
    println!("Running complete hook...");
    hooks.execute("complete", &hook_ctx)?;

    // Execute default merge flow if no custom hook handles it
    if !ctx.config.has_custom_complete_hook() {
        default_merge_flow(&repo_root, &instance.worktree_path, &instance.branch, ctx.name())?;
    }

    // Clean up resources
    cleanup_resources(&ctx.config, &instance, &repo_root)?;

    // Update status
    ctx.set_status(TaskStatus::Completed);
    ctx.save_status()?;

    println!("Task '{}' completed and merged to main.", ctx.name());
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
fn cleanup_resources(config: &WtConfig, instance: &Instance, repo_root: &str) -> Result<()> {
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
