//! Miscellaneous atomic operations for hooks.
//!
//! Usage:
//!   wt internal files:backup <task> [backup_dir]
//!   wt internal files:clean <worktree> <patterns...>
//!   wt internal status:set <task> <status>
//!   wt internal status:get <task>
//!   wt internal task:exists <task>
//!   wt internal task:deps-ready <task>
//!   wt internal task:blocked-by <task>
//!   wt internal notify <title> <message>
//!   wt internal confirm <message>
//!   wt internal abort <message>
//!   wt internal log <task> <message>
//!   wt internal config:get <key>

use crate::error::{Result, WtError};
use crate::models::TaskStore;
use crate::services::{config_ops, files, notify, status_ops};

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
                .map(|i| i.worktree_path.clone())
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

/// Execute a status operation
pub fn execute_status(action: &str, args: Vec<String>) -> Result<()> {
    match action {
        "set" => {
            if args.len() < 2 {
                return Err(WtError::InvalidInput(
                    "status:set requires 2 arguments: <task> <status>".to_string(),
                ));
            }
            let task = &args[0];
            let status = status_ops::parse_status(&args[1])?;
            status_ops::set_status(task, status)
        }
        "get" => {
            if args.is_empty() {
                return Err(WtError::InvalidInput(
                    "status:get requires 1 argument: <task>".to_string(),
                ));
            }
            let status = status_ops::get_status(&args[0])?;
            println!("{}", status);
            Ok(())
        }
        _ => Err(WtError::InvalidInput(format!(
            "Unknown status operation '{}'. Available: set, get",
            action
        ))),
    }
}

/// Execute a task operation
pub fn execute_task(action: &str, args: Vec<String>) -> Result<()> {
    match action {
        "exists" => {
            if args.is_empty() {
                return Err(WtError::InvalidInput(
                    "task:exists requires 1 argument: <task>".to_string(),
                ));
            }
            if status_ops::task_exists(&args[0])? {
                println!("true");
                Ok(())
            } else {
                println!("false");
                std::process::exit(1);
            }
        }
        "deps-ready" => {
            if args.is_empty() {
                return Err(WtError::InvalidInput(
                    "task:deps-ready requires 1 argument: <task>".to_string(),
                ));
            }
            if status_ops::deps_ready(&args[0])? {
                println!("true");
                Ok(())
            } else {
                println!("false");
                std::process::exit(1);
            }
        }
        "blocked-by" => {
            if args.is_empty() {
                return Err(WtError::InvalidInput(
                    "task:blocked-by requires 1 argument: <task>".to_string(),
                ));
            }
            let blocked = status_ops::list_blocked_by(&args[0])?;
            for task in blocked {
                println!("{}", task);
            }
            Ok(())
        }
        _ => Err(WtError::InvalidInput(format!(
            "Unknown task operation '{}'. Available: exists, deps-ready, blocked-by",
            action
        ))),
    }
}

/// Execute a config operation
pub fn execute_config(action: &str, args: Vec<String>) -> Result<()> {
    match action {
        "get" => {
            if args.is_empty() {
                return Err(WtError::InvalidInput(
                    "config:get requires 1 argument: <key>".to_string(),
                ));
            }
            let value = config_ops::get_config(&args[0])?;
            println!("{}", value);
            Ok(())
        }
        _ => Err(WtError::InvalidInput(format!(
            "Unknown config operation '{}'. Available: get",
            action
        ))),
    }
}

/// Execute a notify/interaction operation
pub fn execute_notify(action: &str, args: Vec<String>) -> Result<()> {
    match action {
        "notify" => {
            if args.len() < 2 {
                return Err(WtError::InvalidInput(
                    "notify requires 2 arguments: <title> <message>".to_string(),
                ));
            }
            notify::notify(&args[0], &args[1])
        }
        "confirm" => {
            if args.is_empty() {
                return Err(WtError::InvalidInput(
                    "confirm requires 1 argument: <message>".to_string(),
                ));
            }
            if notify::confirm(&args[0])? {
                Ok(())
            } else {
                std::process::exit(1);
            }
        }
        "abort" => {
            if args.is_empty() {
                return Err(WtError::InvalidInput(
                    "abort requires 1 argument: <message>".to_string(),
                ));
            }
            notify::abort(&args[0])
        }
        "log" => {
            if args.len() < 2 {
                return Err(WtError::InvalidInput(
                    "log requires 2 arguments: <task> <message>".to_string(),
                ));
            }
            notify::log(&args[0], &args[1])
        }
        _ => Err(WtError::InvalidInput(format!(
            "Unknown notify operation '{}'. Available: notify, confirm, abort, log",
            action
        ))),
    }
}
