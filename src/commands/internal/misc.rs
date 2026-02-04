//! Miscellaneous atomic operations for internal commands.
//!
//! Usage:
//!   wt internal files:backup <task> [backup_dir]
//!   wt internal files:clean <worktree> <patterns...>
//!
//! Note: status, task, config, and notify operations were removed in phases-v2.

use crate::error::{Result, WtError};
use crate::models::TaskStore;
use crate::services::files;

/// Execute a files operation
pub fn execute_files(action: &str, args: Vec<String>) -> Result<()> {
    match action {
        "backup" => {
            if args.is_empty() {
                return Err(WtError::InvalidInput(
                    "files:backup requires at least 1 argument: <task> [backup_dir]".to_string(),
                ));
            }
            let task = &args[0];
            let store = TaskStore::load()?;
            let worktree_path = store
                .get_instance(task)
                .and_then(|i| i.worktree_path.clone())
                .ok_or_else(|| {
                    WtError::InvalidInput(format!("Task '{}' has no worktree instance", task))
                })?;

            let backup_dir = args.get(1).map(|s| s.as_str());
            let backup_path = files::backup(task, &worktree_path, backup_dir)?;
            println!("{}", backup_path);
            Ok(())
        }
        "clean" => {
            if args.len() < 2 {
                return Err(WtError::InvalidInput(
                    "files:clean requires at least 2 arguments: <worktree> <patterns...>"
                        .to_string(),
                ));
            }
            let worktree = &args[0];
            let patterns: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
            files::clean(worktree, &patterns)
        }
        _ => Err(WtError::InvalidInput(format!(
            "Unknown files operation '{}'. Available: backup, clean",
            action
        ))),
    }
}

/// Execute a status operation (removed in phases-v2)
pub fn execute_status(action: &str, _args: Vec<String>) -> Result<()> {
    Err(WtError::InvalidInput(format!(
        "status:{} operation was removed in phases-v2. Use 'wt status' command instead.",
        action
    )))
}

/// Execute a task operation (removed in phases-v2)
pub fn execute_task(action: &str, _args: Vec<String>) -> Result<()> {
    Err(WtError::InvalidInput(format!(
        "task:{} operation was removed in phases-v2.",
        action
    )))
}

/// Execute a config operation (removed in phases-v2)
pub fn execute_config(action: &str, _args: Vec<String>) -> Result<()> {
    Err(WtError::InvalidInput(format!(
        "config:{} operation was removed in phases-v2.",
        action
    )))
}

/// Execute a notify/interaction operation (removed in phases-v2)
pub fn execute_notify(action: &str, _args: Vec<String>) -> Result<()> {
    Err(WtError::InvalidInput(format!(
        "{} operation was removed in phases-v2.",
        action
    )))
}
