use std::path::Path;

use crate::error::{Result, WtError};
use crate::models::{TaskStatus, TaskStore, WtConfig};
use crate::services::{git, tmux, workspace::WorkspaceInitializer};

pub fn execute(task_ref: String, silent: bool) -> Result<()> {
    let config = WtConfig::load()?;
    let mut store = TaskStore::load()?;

    // Resolve task reference (name or index) to actual name
    let name = store.resolve_task_ref(&task_ref)?;

    let is_scratch = store.is_scratch(&name);
    let current_status = store.get_status(&name);

    if is_scratch {
        // Scratch: allow from Running directly (skip Merged requirement)
        if current_status != TaskStatus::Running && current_status != TaskStatus::Merged {
            return Err(WtError::InvalidStateTransition {
                from: current_status.display_name().to_string(),
                to: "archived".to_string(),
            });
        }
    } else {
        // Normal task: check task file exists and validate transition
        store.ensure_exists(&name)?;
        store.validate_transition(&name, TaskStatus::Archived)?;
    }

    // Get instance info and repo root before modifying anything
    let instance = store.get_instance(&name).cloned();
    let repo_root = git::get_repo_root()?;

    // Run archive script if configured (before any cleanup)
    if let Some(ref script) = config.archive_script {
        if let Some(ref inst) = instance {
            if !silent {
                println!("Running archive script...");
            }
            let source_dir = Path::new(".");
            let initializer = WorkspaceInitializer::new(&inst.worktree_path, source_dir);
            initializer.run_init_script(script)?;
        }
    }

    // Update status BEFORE deleting worktree (symlink would be deleted with worktree)
    if is_scratch {
        store.status.tasks.remove(&name);
    } else {
        store.set_status(&name, TaskStatus::Archived);
        store.set_instance(&name, None);
    }
    store.save_status()?;

    // Cleanup all resources (after status is saved)
    if let Some(inst) = instance {
        if !silent {
            println!("Archiving resources...");
        }

        // Kill tmux session (may already be gone from done/merged)
        let _ = tmux::kill_session(&inst.tmux_session);

        // Remove worktree
        if let Err(e) = git::remove_worktree(&inst.worktree_path) {
            if !silent {
                eprintln!("  Warning: Failed to remove worktree: {}", e);
            }
        } else if !silent {
            println!("  Removed worktree: {}", inst.worktree_path);
        }

        // Delete branch (run from repo root since worktree is gone)
        if let Err(e) = git::delete_branch_in(&inst.branch, &repo_root) {
            if !silent {
                eprintln!("  Warning: Failed to delete branch: {}", e);
            }
        } else if !silent {
            println!("  Deleted branch: {}", inst.branch);
        }
    }

    if !silent {
        if is_scratch {
            println!("Scratch environment '{}' cleaned up.", name);
        } else {
            println!("Task '{}' archived.", name);
        }
    }
    Ok(())
}
