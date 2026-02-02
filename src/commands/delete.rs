//! Delete command - remove task resources (worktree, branch).

use std::path::Path;

use crate::error::{Result, WtError};
use crate::models::{HookContext, TaskStatus, TaskStore, WtConfig};
use crate::services::{git, hooks::HooksEngine, multiplexer::create_multiplexer};

/// Execute the delete command.
///
/// Deletes a task's resources (worktree, branch) based on its status:
/// - Completed task: directly delete resources, keep completed status record
/// - Review/Running task: requires --force, after deletion status returns to Pending
/// - Pending task: error (no resources to delete)
/// - Scratch (Running/Review): directly delete, remove all records
/// - Scratch (Pending/Completed): error (invalid state)
pub fn execute(task_ref: String, force: bool) -> Result<()> {
    let config = WtConfig::load()?;
    let mut store = TaskStore::load()?;

    // Resolve task reference (name or index) to actual name
    let name = store.resolve_task_ref(&task_ref)?;

    let is_scratch = store.is_scratch(&name);
    let current_status = store.get_status(&name);

    // Check if we can delete based on status
    if is_scratch {
        // Scratch: only allow from Running or Review
        if current_status != TaskStatus::Running && current_status != TaskStatus::Review {
            return Err(WtError::InvalidStateTransition {
                from: current_status.display_name().to_string(),
                to: "deleted".to_string(),
            });
        }
    } else {
        // Normal task
        match (&current_status, force) {
            // Completed tasks can be directly deleted
            (TaskStatus::Completed, _) => {}

            // Pending tasks have no resources to delete
            (TaskStatus::Pending, _) => {
                return Err(WtError::InvalidInput(format!(
                    "Task '{}' is pending, no resources to delete. Use 'wt start {}' to create resources first.",
                    name, name
                )));
            }

            // Running/Review with force
            (status, true) => {
                eprintln!(
                    "Warning: Force deleting task '{}' in {} state.",
                    name,
                    status.display_name()
                );
            }

            // Running/Review without force
            (status, false) => {
                return Err(WtError::InvalidInput(format!(
                    "Task '{}' is in {} state. Use --force to delete, or complete it first with 'wt merge {}'.",
                    name, status.display_name(), name
                )));
            }
        }
    }

    // Get instance info and repo root before modifying anything
    let instance = store.get_instance(&name).cloned();
    let repo_root = git::get_repo_root()?;

    // Build hook context
    let context = build_hook_context(&config, &store, &name, &repo_root)?;

    // Run before_delete hook
    let hooks = HooksEngine::new(&config);
    hooks.before_delete(&context)?;

    // Close multiplexer window if exists
    if let Some(ref inst) = instance {
        let mux = create_multiplexer(inst.multiplexer_type());
        let _ = mux.kill_window(&inst.session_name, &inst.window_name);
    }

    // Delete worktree and branch
    if let Some(ref inst) = instance {
        if is_scratch {
            println!("Deleting scratch environment...");
        }

        let worktree_path = Path::new(&inst.worktree_path);
        if worktree_path.exists() {
            if let Err(e) = git::remove_worktree(&inst.worktree_path) {
                eprintln!("  Warning: Failed to remove worktree: {}", e);
            } else {
                println!("  Removed worktree: {}", inst.worktree_path);
            }
        }

        // Delete branch (from repo root since worktree is gone)
        if let Err(e) = git::delete_branch(&repo_root, &inst.branch) {
            eprintln!("  Warning: Failed to delete branch: {}", e);
        } else {
            println!("  Deleted branch: {}", inst.branch);
        }
    }

    // Update status based on task type
    if is_scratch {
        // Scratch: remove all records
        store.status.tasks.remove(&name);
        store.save_status()?;
        println!("Scratch environment '{}' deleted.", name);
    } else {
        // Clear instance data
        store.set_instance(&name, None);

        // For non-completed tasks that were force-deleted, reset to Pending
        if current_status != TaskStatus::Completed {
            store.set_status(&name, TaskStatus::Pending);
        }
        // For completed tasks, keep the Completed status

        store.save_status()?;
        println!("Deleted resources for '{}'.", name);
    }

    Ok(())
}

/// Build a HookContext for the delete operation.
fn build_hook_context(
    config: &WtConfig,
    store: &TaskStore,
    name: &str,
    repo_root: &str,
) -> Result<HookContext> {
    let instance = store.get_instance(name);
    let status = store.get_status(name);

    let (branch, worktree) = if let Some(inst) = instance {
        (inst.branch.clone(), inst.worktree_path.clone())
    } else {
        (format!("wt/{}", name), String::new())
    };

    let (session, window) = if let Some(inst) = instance {
        (inst.session_name.clone(), inst.window_name.clone())
    } else {
        (config.session_name.clone(), name.to_string())
    };

    Ok(HookContext::new(name, &branch, &worktree, repo_root)
        .with_session(&session)
        .with_window(&window)
        .with_status(status.display_name()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_hook_context_minimal() {
        let config = WtConfig::from_str("session_name: test\n").unwrap();
        let store = TaskStore::default();

        let ctx = build_hook_context(&config, &store, "task1", "/repo").unwrap();

        assert_eq!(ctx.task, "task1");
        assert_eq!(ctx.branch, "wt/task1");
        assert_eq!(ctx.repo_root, "/repo");
        assert_eq!(ctx.session, "test");
        assert_eq!(ctx.window, "task1");
        assert_eq!(ctx.status, "pending");
    }
}
