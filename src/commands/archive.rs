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

    // Run archive script if configured
    if let Some(ref script) = config.archive_script {
        if let Some(instance) = store.get_instance(&name) {
            if !silent {
                println!("Running archive script...");
            }
            let source_dir = Path::new(".");
            let initializer = WorkspaceInitializer::new(&instance.worktree_path, source_dir);
            initializer.run_init_script(script)?;
        }
    }

    // Cleanup all resources
    if let Some(instance) = store.get_instance(&name) {
        if !silent {
            println!("Archiving resources...");
        }

        // Kill tmux window (may already be gone from merged)
        let _ = tmux::kill_window(&instance.tmux_session, &instance.tmux_window);

        // Remove worktree
        if let Err(e) = git::remove_worktree(&instance.worktree_path) {
            if !silent {
                eprintln!("  Warning: Failed to remove worktree: {}", e);
            }
        } else if !silent {
            println!("  Removed worktree: {}", instance.worktree_path);
        }

        // Delete branch
        if let Err(e) = git::delete_branch(&instance.branch) {
            if !silent {
                eprintln!("  Warning: Failed to delete branch: {}", e);
            }
        } else if !silent {
            println!("  Deleted branch: {}", instance.branch);
        }
    }

    // Update status
    if is_scratch {
        // Scratch: remove entry from status.json entirely
        store.status.tasks.remove(&name);
        store.save_status()?;
        if !silent {
            println!("Scratch environment '{}' cleaned up.", name);
        }
    } else {
        // Normal task: set to Archived and clear instance
        store.set_status(&name, TaskStatus::Archived);
        store.set_instance(&name, None);
        store.save_status()?;
        if !silent {
            println!("Task '{}' archived.", name);
        }
    }
    Ok(())
}
