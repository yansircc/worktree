//! Task reference resolution logic.
//!
//! Resolves task references (name or 1-based index) to actual task names.

use crate::error::{Result, WtError};
use crate::models::Task;
use std::collections::HashMap;

/// Task resolver that handles name and index lookups.
pub struct TaskResolver<'a> {
    /// Task files (name -> Task)
    tasks: &'a HashMap<String, Task>,
    /// Status entries (for scratch environments)
    status_names: &'a std::collections::HashSet<String>,
}

impl<'a> TaskResolver<'a> {
    /// Create a new resolver with task files and status names.
    pub fn new(
        tasks: &'a HashMap<String, Task>,
        status_names: &'a std::collections::HashSet<String>,
    ) -> Self {
        Self {
            tasks,
            status_names,
        }
    }

    /// Resolve a task reference (name or 1-based index) to a task name.
    ///
    /// Priority:
    /// 1. Exact name match in task files
    /// 2. Exact name match in status.json (scratch environments)
    /// 3. Numeric index (1-based)
    pub fn resolve(&self, task_ref: &str) -> Result<String> {
        // Priority 1: exact name match in task files
        if self.tasks.contains_key(task_ref) {
            return Ok(task_ref.to_string());
        }

        // Priority 2: exact name match in status.json (scratch environments)
        if self.status_names.contains(task_ref) {
            return Ok(task_ref.to_string());
        }

        // Priority 3: try numeric index (1-based)
        if let Ok(index) = task_ref.parse::<usize>() {
            return self.get_name_by_index(index);
        }

        Err(WtError::TaskNotFound(task_ref.to_string()))
    }

    /// Get task name by 1-based index (sorted alphabetically).
    fn get_name_by_index(&self, index: usize) -> Result<String> {
        let tasks = self.list_sorted();
        let total = tasks.len();
        if index == 0 || index > total {
            return Err(WtError::InvalidTaskIndex { index, total });
        }
        Ok(tasks[index - 1].name().to_string())
    }

    /// List all tasks sorted by name.
    fn list_sorted(&self) -> Vec<&Task> {
        let mut tasks: Vec<_> = self.tasks.values().collect();
        tasks.sort_by(|a, b| a.name().cmp(b.name()));
        tasks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TaskFrontmatter;
    use std::collections::HashSet;

    fn create_test_task(name: &str) -> Task {
        Task {
            frontmatter: TaskFrontmatter {
                name: name.to_string(),
                depends: vec![],
            },
            content: String::new(),
            file_path: format!("{}.md", name),
        }
    }

    fn create_tasks(names: &[&str]) -> HashMap<String, Task> {
        names
            .iter()
            .map(|&name| (name.to_string(), create_test_task(name)))
            .collect()
    }

    #[test]
    fn test_resolve_by_name() {
        let tasks = create_tasks(&["alpha", "beta"]);
        let status_names = HashSet::new();
        let resolver = TaskResolver::new(&tasks, &status_names);

        assert_eq!(resolver.resolve("alpha").unwrap(), "alpha");
        assert_eq!(resolver.resolve("beta").unwrap(), "beta");
    }

    #[test]
    fn test_resolve_by_index() {
        let tasks = create_tasks(&["alpha", "beta", "gamma"]);
        let status_names = HashSet::new();
        let resolver = TaskResolver::new(&tasks, &status_names);

        // Tasks are sorted alphabetically: alpha=1, beta=2, gamma=3
        assert_eq!(resolver.resolve("1").unwrap(), "alpha");
        assert_eq!(resolver.resolve("2").unwrap(), "beta");
        assert_eq!(resolver.resolve("3").unwrap(), "gamma");
    }

    #[test]
    fn test_resolve_name_priority_over_index() {
        // Create a task named "2" - name should take priority over index
        let tasks = create_tasks(&["1", "2", "alpha"]);
        let status_names = HashSet::new();
        let resolver = TaskResolver::new(&tasks, &status_names);

        // "1" matches task name first
        let result = resolver.resolve("1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "1");
    }

    #[test]
    fn test_resolve_scratch_environment() {
        let tasks = create_tasks(&["alpha"]);
        let mut status_names = HashSet::new();
        status_names.insert("scratch-1".to_string());
        let resolver = TaskResolver::new(&tasks, &status_names);

        assert_eq!(resolver.resolve("scratch-1").unwrap(), "scratch-1");
    }

    #[test]
    fn test_resolve_index_zero_error() {
        let tasks = create_tasks(&["alpha"]);
        let status_names = HashSet::new();
        let resolver = TaskResolver::new(&tasks, &status_names);

        let result = resolver.resolve("0");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid task index 0"));
        assert!(err.contains("valid range is 1-1"));
    }

    #[test]
    fn test_resolve_index_out_of_range() {
        let tasks = create_tasks(&["alpha", "beta"]);
        let status_names = HashSet::new();
        let resolver = TaskResolver::new(&tasks, &status_names);

        let result = resolver.resolve("99");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid task index 99"));
        assert!(err.contains("valid range is 1-2"));
    }

    #[test]
    fn test_resolve_not_found() {
        let tasks = create_tasks(&["alpha"]);
        let status_names = HashSet::new();
        let resolver = TaskResolver::new(&tasks, &status_names);

        let result = resolver.resolve("nonexistent");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("nonexistent"));
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_resolve_empty_store() {
        let tasks = HashMap::new();
        let status_names = HashSet::new();
        let resolver = TaskResolver::new(&tasks, &status_names);

        let result = resolver.resolve("1");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("valid range is 1-0"));
    }
}
