use crate::models::{TaskStatus, TaskStore};

/// Find all tasks that depend on the given task and are not in Pending state.
/// Returns a list of (task_name, status) pairs.
pub fn find_non_pending_dependents(
    store: &TaskStore,
    task_name: &str,
) -> Vec<(String, TaskStatus)> {
    let mut result = Vec::new();

    for task in store.list() {
        if task.depends().contains(&task_name.to_string()) {
            let status = store.status.get_status(task.name());
            if status != TaskStatus::Pending {
                result.push((task.name().to_string(), status));
            }
        }
    }

    result
}
