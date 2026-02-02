//! Status operations for hooks system.

use crate::error::{Result, WtError};
use crate::models::{StatusStore, TaskStatus, TaskStore};

/// Set task status.
pub fn set_status(task: &str, status: TaskStatus) -> Result<()> {
    let mut store = StatusStore::load()?;
    store.set_status(task, status);
    store.save()?;
    Ok(())
}

/// Get task status as string (for scripts).
pub fn get_status(task: &str) -> Result<String> {
    let store = StatusStore::load()?;
    let status = store.get_status(task);
    Ok(status.display_name().to_string())
}

/// Check if a task exists (either as task file or in status).
pub fn task_exists(task: &str) -> Result<bool> {
    let store = TaskStore::load()?;
    // Check task files
    if store.tasks.contains_key(task) {
        return Ok(true);
    }
    // Check status.json (scratch environments)
    if store.status.tasks.contains_key(task) {
        return Ok(true);
    }
    Ok(false)
}

/// Check if all dependencies of a task are completed.
pub fn deps_ready(task: &str) -> Result<bool> {
    let store = TaskStore::load()?;

    let task_def = store
        .get(task)
        .ok_or_else(|| WtError::TaskNotFound(task.to_string()))?;

    for dep_name in task_def.depends() {
        let dep_status = store.get_status(dep_name);
        if dep_status != TaskStatus::Completed {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Get tasks that are blocked by the given task (i.e., depend on it).
pub fn list_blocked_by(task: &str) -> Result<Vec<String>> {
    let store = TaskStore::load()?;

    // Ensure task exists
    if !store.tasks.contains_key(task) && !store.status.tasks.contains_key(task) {
        return Err(WtError::TaskNotFound(task.to_string()));
    }

    let mut blocked = Vec::new();

    for t in store.tasks.values() {
        if t.depends().contains(&task.to_string()) {
            let status = store.get_status(t.name());
            // Only include tasks that would be affected (not completed)
            if status != TaskStatus::Completed {
                blocked.push(t.name().to_string());
            }
        }
    }

    blocked.sort();
    Ok(blocked)
}

/// Parse status string to TaskStatus enum.
pub fn parse_status(status_str: &str) -> Result<TaskStatus> {
    match status_str.to_lowercase().as_str() {
        "pending" => Ok(TaskStatus::Pending),
        "active" => Ok(TaskStatus::Active),
        "idle" => Ok(TaskStatus::Idle),
        "completed" => Ok(TaskStatus::Completed),
        _ => Err(WtError::InvalidInput(format!(
            "Invalid status '{}'. Valid values: pending, active, idle, completed",
            status_str
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_status_valid() {
        assert_eq!(parse_status("pending").unwrap(), TaskStatus::Pending);
        assert_eq!(parse_status("active").unwrap(), TaskStatus::Active);
        assert_eq!(parse_status("idle").unwrap(), TaskStatus::Idle);
        assert_eq!(parse_status("completed").unwrap(), TaskStatus::Completed);
    }

    #[test]
    fn test_parse_status_case_insensitive() {
        assert_eq!(parse_status("PENDING").unwrap(), TaskStatus::Pending);
        assert_eq!(parse_status("Active").unwrap(), TaskStatus::Active);
        assert_eq!(parse_status("IDLE").unwrap(), TaskStatus::Idle);
    }

    #[test]
    fn test_parse_status_invalid() {
        let result = parse_status("invalid");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid status"));
    }
}
