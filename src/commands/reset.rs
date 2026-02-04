//! Reset command - reset a task to a specific phase.
//!
//! Usage:
//! - `wt reset <task>` - Reset to pending (default)
//! - `wt reset <task> --to developing` - Reset to developing phase
//!
//! Behavior:
//! 1. Validates dependencies (non-pending dependents block reset)
//! 2. Runs reset hook (cleanup scripts)
//! 3. Backs up worktree (unless scratch)
//! 4. Cleans up resources (window, worktree, branch) if resetting to pending
//! 5. Updates status to target phase

use std::env;
use std::fs;
use std::path::Path;

use chrono::Utc;

use crate::constants::{branch_pattern, BACKUPS_DIR};
use crate::error::{Result, WtError};
use crate::models::{TaskStatus, WtConfig};
use crate::services::{dependency, git, multiplexer::create_multiplexer, TaskContext};

/// Validate and normalize target phase from string.
/// Returns None for "none" keyword or phases with no resources and not terminal,
/// Some(phase_id) for other phases.
fn validate_target_phase(phase_str: &str, config: &WtConfig) -> Result<Option<String>> {
    let normalized = phase_str.to_lowercase();

    // "none" is a special keyword meaning "reset to no phase"
    if normalized == "none" {
        return Ok(None);
    }

    // Check if phase exists in sequence
    let seq = config.phase_sequence()?;
    if !seq.iter().any(|s| s == &normalized) {
        return Err(crate::error::WtError::InvalidInput(format!(
            "Invalid phase '{}'. Valid phases: {}",
            phase_str,
            seq.join(", ")
        )));
    }

    // Check phase definition: no resources + not terminal = back to initial state
    if let Some(phase) = config.get_phase(&normalized) {
        if phase.resources.is_empty() && !phase.terminal {
            return Ok(None);
        }
    }

    Ok(Some(normalized))
}

pub fn execute(task_ref: String, to_phase: Option<String>) -> Result<()> {
    let config = WtConfig::load()?;

    // Parse and validate target phase
    // None means reset to pending (first phase), Some(id) means reset to that phase
    let target_phase: Option<String> = if let Some(phase_str) = &to_phase {
        validate_target_phase(phase_str, &config)?
    } else {
        None // Default: reset to pending
    };

    // If target is None (pending), we do a "full reset" (clean up resources)
    let full_reset = target_phase.is_none();
    let mut ctx = TaskContext::load(&task_ref)?;

    let is_scratch = ctx.is_scratch();
    let name = ctx.name().to_string();

    // For normal tasks, check task file exists
    if !is_scratch {
        ctx.store.ensure_exists(&name)?;
    }

    let current_status = ctx.status();

    // For scratch, skip dependent check (no other tasks depend on scratch)
    // For normal tasks, check for non-pending dependents
    if !is_scratch && current_status != TaskStatus::Pending {
        let dependents: Vec<_> = dependency::find_non_pending_dependents(&ctx.store, &name)
            .into_iter()
            .filter(|(_, status)| *status != TaskStatus::Completed)
            .collect();
        if let Some((dep_name, dep_status)) = dependents.first() {
            return Err(WtError::HasDependents {
                task: name.clone(),
                dependent: dep_name.clone(),
                status: dep_status.display_name().to_string(),
            });
        }
    }

    // For soft reset (non-pending target), skip resource cleanup
    if !full_reset {
        // Just update the phase
        return update_status_only(&mut ctx, &name, is_scratch, target_phase);
    }

    // Get repo root before cleanup (needed for git commands after worktree removal)
    let repo_root = ctx.repo_root()?.to_string();

    // Backup and cleanup resources if instance exists (full reset only)
    if let Some(instance) = ctx.instance().cloned() {
        // Backup worktree before cleanup (skip for scratch environments)
        if let Some(ref wt_path) = instance.worktree_path {
            if Path::new(wt_path).exists() && !is_scratch {
                backup_worktree(&name, wt_path)?;
            }
        }

        println!("Cleaning up resources...");

        // Kill multiplexer window
        if let Some(ref window) = instance.window_name {
            let mux = create_multiplexer(instance.multiplexer_type());
            if let Err(e) = mux.kill_window(&instance.session_name, window) {
                eprintln!(
                    "  Warning: Failed to kill {} window: {}",
                    instance.multiplexer, e
                );
            } else {
                println!(
                    "  Killed {} window: {}:{}",
                    instance.multiplexer, instance.session_name, window
                );
            }
        }

        // Remove worktree
        if let Some(ref wt_path) = instance.worktree_path {
            if let Err(e) = git::remove_worktree(wt_path) {
                eprintln!("  Warning: Failed to remove worktree: {}", e);
            } else {
                println!("  Removed worktree: {}", wt_path);
            }
        }

        // Delete branch (run from repo root since worktree is gone)
        if let Some(ref branch) = instance.branch {
            if let Err(e) = git::delete_branch_in(branch, &repo_root) {
                eprintln!("  Warning: Failed to delete branch: {}", e);
            } else {
                println!("  Deleted branch: {}", branch);
            }
        }
    } else {
        // No instance saved, but there might be orphaned resources from a failed start
        // Try to clean up based on expected paths
        let cleaned = cleanup_orphaned_resources(&name, &ctx.config, &repo_root)?;

        // If already pending and nothing to clean, just report
        if current_status == TaskStatus::Pending && !cleaned {
            println!("Task '{}' is already pending.", name);
            return Ok(());
        }
    }

    // Update status
    if is_scratch {
        // Scratch: remove entry from status.json entirely
        ctx.store.status.tasks.remove(&name);
        ctx.save_status()?;
        println!("Scratch environment '{}' cleaned up.", name);
    } else if full_reset {
        // Full reset: reset to Pending and clear instance
        ctx.set_status(TaskStatus::Pending);
        ctx.store.status.set_instance(&name, None);
        ctx.state_mut().phase = None;
        ctx.save_status()?;
        println!("Task '{}' reset to pending.", name);
    } else {
        // Soft reset: just change phase, keep resources
        let phase_display = target_phase.clone().unwrap_or_else(|| "(initial)".to_string());
        ctx.state_mut().phase = target_phase;
        ctx.state_mut().status = TaskStatus::Idle;
        ctx.state_mut().step_result = None;
        ctx.state_mut().active_since = None;
        ctx.save_status()?;
        println!(
            "Task '{}' reset to phase '{}'.",
            name,
            phase_display
        );
    }
    Ok(())
}

/// Soft reset: just update status without cleaning resources
fn update_status_only(
    ctx: &mut TaskContext,
    name: &str,
    is_scratch: bool,
    target_phase: Option<String>,
) -> Result<()> {
    if is_scratch {
        return Err(WtError::InvalidInput(
            "Cannot soft-reset scratch environments. Use 'wt reset <name>' for full cleanup."
                .to_string(),
        ));
    }

    let phase_display = target_phase.clone().unwrap_or_else(|| "(initial)".to_string());
    ctx.state_mut().phase = target_phase;
    ctx.state_mut().status = TaskStatus::Idle;
    ctx.state_mut().step_result = None;
    ctx.state_mut().active_since = None;
    ctx.save_status()?;

    println!(
        "Task '{}' reset to phase '{}'.",
        name,
        phase_display
    );
    println!(
        "Hint: Resources (worktree, branch) were kept. Run 'wt next {}' to continue.",
        name
    );

    Ok(())
}

/// Clean up orphaned resources from a failed start (no instance saved)
/// Returns true if any resources were cleaned up
fn cleanup_orphaned_resources(task_name: &str, config: &WtConfig, repo_root: &str) -> Result<bool> {
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

    // Delete any matching branches (run from repo root since worktree may be gone)
    for branch in branches {
        if let Err(e) = git::delete_branch_in(&branch, repo_root) {
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
