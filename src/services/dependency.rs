use crate::error::{Result, WtError};
use crate::models::{TaskStatus, TaskStore};

pub fn check_dependencies_completed(store: &TaskStore, task_name: &str) -> Result<()> {
    let task = store
        .get(task_name)
        .ok_or_else(|| WtError::TaskNotFound(task_name.to_string()))?;

    for dep_name in task.depends() {
        // Check dependency exists
        let _dep = store
            .get(dep_name)
            .ok_or_else(|| WtError::DependencyNotFound(dep_name.clone()))?;

        // Check dependency status from StatusStore
        let dep_status = store.get_status(dep_name);
        if dep_status != TaskStatus::Completed {
            return Err(WtError::DependencyNotCompleted {
                task: task_name.to_string(),
                dep: dep_name.clone(),
            });
        }
    }
    Ok(())
}

/// Find all tasks that depend on the given task and are not in Pending state.
/// Returns a list of (task_name, status) pairs.
pub fn find_non_pending_dependents(store: &TaskStore, task_name: &str) -> Vec<(String, TaskStatus)> {
    let mut result = Vec::new();

    for task in store.list() {
        if task.depends().contains(&task_name.to_string()) {
            let status = store.get_status(task.name());
            if status != TaskStatus::Pending {
                result.push((task.name().to_string(), status));
            }
        }
    }

    result
}
