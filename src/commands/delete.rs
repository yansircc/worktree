//! Delete command - remove task resources (worktree, branch).

use std::path::Path;

use crate::error::{Result, WtError};
use crate::models::TaskStatus;
use crate::services::{git, multiplexer::create_multiplexer, TaskContext};

/// Execute the delete command.
///
/// Deletes a task's resources (worktree, branch) based on its status:
/// - Completed task: directly delete resources, keep completed status record
/// - Idle/Active task: requires --force, after deletion status returns to Pending
/// - Pending task: error (no resources to delete)
/// - Scratch (Active/Idle): directly delete, remove all records
/// - Scratch (Pending/Completed): error (invalid state)
pub fn execute(task_ref: String, force: bool) -> Result<()> {
    let mut ctx = TaskContext::load(&task_ref)?;

    let is_scratch = ctx.is_scratch();
    let current_status = ctx.status();
    let name = ctx.name().to_string();

    // Check if we can delete based on status
    if is_scratch {
        // Scratch: only allow from Active or Idle
        if current_status != TaskStatus::Active && current_status != TaskStatus::Idle {
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
                    "Task '{}' is pending, no resources to delete. Use 'wt run {}' to create resources first.",
                    name, name
                )));
            }

            // Active/Idle with force
            (status, true) => {
                eprintln!(
                    "Warning: Force deleting task '{}' in {} state.",
                    name,
                    status.display_name()
                );
            }

            // Active/Idle without force
            (status, false) => {
                return Err(WtError::InvalidInput(format!(
                    "Task '{}' is in {} state. Use --force to delete, or complete it first with 'wt complete {}'.",
                    name, status.display_name(), name
                )));
            }
        }
    }

    // Get instance info before modifying anything
    let instance = ctx.instance().cloned();
    let repo_root = ctx.repo_root()?.to_string();

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
        ctx.store.status.tasks.remove(&name);
        ctx.save_status()?;
        println!("Scratch environment '{}' deleted.", name);
    } else {
        // Clear instance data
        ctx.store.set_instance(&name, None);

        // For non-completed tasks that were force-deleted, reset to Pending
        if current_status != TaskStatus::Completed {
            ctx.set_status(TaskStatus::Pending);
        }
        // For completed tasks, keep the Completed status

        ctx.save_status()?;
        println!("Deleted resources for '{}'.", name);
    }

    Ok(())
}
