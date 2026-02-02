use std::env;
use std::fs;
use std::path::Path;

use chrono::Utc;

use crate::constants::{branch_pattern, BACKUPS_DIR};
use crate::error::{Result, WtError};
use crate::models::{TaskStatus, TaskStore, WtConfig};
use crate::services::{dependency, git, tmux, workspace::WorkspaceInitializer};

pub fn execute(task_ref: String) -> Result<()> {
    let config = WtConfig::load()?;
    let mut store = TaskStore::load()?;

    // Resolve task reference (name or index) to actual name
    let name = store.resolve_task_ref(&task_ref)?;

    let is_scratch = store.is_scratch(&name);

    // For normal tasks, check task file exists
    if !is_scratch {
        store.ensure_exists(&name)?;
    }

    let current_status = store.get_status(&name);

    // For scratch, skip dependent check (no other tasks depend on scratch)
    // For normal tasks, check for non-pending dependents
    if !is_scratch && current_status != TaskStatus::Pending {
        let dependents: Vec<_> = dependency::find_non_pending_dependents(&store, &name)
            .into_iter()
            .filter(|(_, status)| *status != TaskStatus::Archived)
            .collect();
        if let Some((dep_name, dep_status)) = dependents.first() {
            return Err(WtError::HasDependents {
                task: name.clone(),
                dependent: dep_name.clone(),
                status: dep_status.display_name().to_string(),
            });
        }
    }

    // Backup and cleanup resources if instance exists
    if let Some(instance) = store.get_instance(&name).cloned() {
        let worktree_path = Path::new(&instance.worktree_path);

        // Run archive_script to slim down before backup (skip for scratch)
        if worktree_path.exists() {
            if let Some(ref script) = config.archive_script {
                println!("Running archive script...");
                let source_dir = Path::new(".");
                let initializer = WorkspaceInitializer::new(&instance.worktree_path, source_dir);
                if let Err(e) = initializer.run_init_script(script) {
                    eprintln!("  Warning: Archive script failed: {}", e);
                }
            }

            // Skip backup for scratch environments
            if !is_scratch {
                backup_worktree(&name, &instance.worktree_path)?;
            }
        }

        println!("Cleaning up resources...");

        // Kill tmux window
        if let Err(e) = tmux::kill_window(&instance.tmux_session, &instance.tmux_window) {
            eprintln!("  Warning: Failed to kill tmux window: {}", e);
        } else {
            println!("  Killed tmux window: {}:{}", instance.tmux_session, instance.tmux_window);
        }

        // Remove worktree
        if let Err(e) = git::remove_worktree(&instance.worktree_path) {
            eprintln!("  Warning: Failed to remove worktree: {}", e);
        } else {
            println!("  Removed worktree: {}", instance.worktree_path);
        }

        // Delete branch
        if let Err(e) = git::delete_branch(&instance.branch) {
            eprintln!("  Warning: Failed to delete branch: {}", e);
        } else {
            println!("  Deleted branch: {}", instance.branch);
        }
    } else {
        // No instance saved, but there might be orphaned resources from a failed start
        // Try to clean up based on expected paths
        let cleaned = cleanup_orphaned_resources(&name, &config)?;

        // If already pending and nothing to clean, just report
        if current_status == TaskStatus::Pending && !cleaned {
            println!("Task '{}' is already pending.", name);
            return Ok(());
        }
    }

    // Update status
    if is_scratch {
        // Scratch: remove entry from status.json entirely
        store.status.tasks.remove(&name);
        store.save_status()?;
        println!("Scratch environment '{}' cleaned up.", name);
    } else {
        // Normal task: reset to Pending and clear instance
        store.set_status(&name, TaskStatus::Pending);
        store.set_instance(&name, None);
        store.save_status()?;
        println!("Task '{}' reset to pending.", name);
    }
    Ok(())
}

/// Clean up orphaned resources from a failed start (no instance saved)
/// Returns true if any resources were cleaned up
fn cleanup_orphaned_resources(task_name: &str, config: &WtConfig) -> Result<bool> {
    let cwd = env::current_dir().map_err(|e| WtError::Git(e.to_string()))?;
    let worktree_path = cwd
        .join(&config.worktree_dir)
        .join(task_name)
        .to_string_lossy()
        .to_string();

    let worktree_exists = Path::new(&worktree_path).exists();
    let branches = git::find_branches(&branch_pattern(task_name));

    if !worktree_exists && branches.is_empty() {
        return Ok(false); // Nothing to clean up
    }

    println!("Cleaning up orphaned resources...");

    // Remove worktree if exists
    if worktree_exists {
        if let Err(e) = git::remove_worktree(&worktree_path) {
            eprintln!("  Warning: Failed to remove worktree: {}", e);
        } else {
            println!("  Removed worktree: {}", worktree_path);
        }
    }

    // Delete any matching branches
    for branch in branches {
        if let Err(e) = git::delete_branch(&branch) {
            eprintln!("  Warning: Failed to delete branch {}: {}", branch, e);
        } else {
            println!("  Deleted branch: {}", branch);
        }
    }

    Ok(true)
}

fn backup_worktree(task_name: &str, worktree_path: &str) -> Result<()> {
    let source = Path::new(worktree_path);
    if !source.exists() {
        return Ok(()); // Nothing to backup
    }

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let backup_dir = Path::new(BACKUPS_DIR);
    fs::create_dir_all(backup_dir).map_err(|e| WtError::Io {
        operation: "create backup directory".to_string(),
        path: BACKUPS_DIR.to_string(),
        message: e.to_string(),
    })?;

    let backup_name = format!("{}-{}", task_name, timestamp);
    let backup_path = backup_dir.join(&backup_name);

    // Copy directory recursively (exclude .git)
    copy_dir_recursive(source, &backup_path)?;

    println!("  Backed up worktree to: {}", backup_path.display());
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).map_err(|e| WtError::Io {
        operation: "create backup".to_string(),
        path: dst.to_string_lossy().to_string(),
        message: e.to_string(),
    })?;

    for entry in fs::read_dir(src).map_err(|e| WtError::Io {
        operation: "read directory".to_string(),
        path: src.to_string_lossy().to_string(),
        message: e.to_string(),
    })? {
        let entry = entry.map_err(|e| WtError::Io {
            operation: "read entry".to_string(),
            path: src.to_string_lossy().to_string(),
            message: e.to_string(),
        })?;
        let path = entry.path();
        let file_name = path.file_name().unwrap();

        // Skip .git directory (it's a link to main repo's .git)
        if file_name == ".git" {
            continue;
        }

        let dest_path = dst.join(file_name);
        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path).map_err(|e| WtError::Io {
                operation: "copy file".to_string(),
                path: path.to_string_lossy().to_string(),
                message: e.to_string(),
            })?;
        }
    }
    Ok(())
}
