//! Task validation logic.
//!
//! This module contains validation functions for tasks, including:
//! - Dependency existence checks
//! - Filename/name matching
//! - Circular dependency detection

use std::collections::HashSet;
use std::path::Path;

use super::Task;

/// Validation error for a task
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub task: String,
    pub message: String,
}

impl ValidationError {
    pub fn new(task: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            task: task.into(),
            message: message.into(),
        }
    }
}

/// Task validator that checks task configurations
pub struct TaskValidator<'a> {
    tasks: &'a std::collections::HashMap<String, Task>,
}

impl<'a> TaskValidator<'a> {
    /// Create a new validator for the given tasks
    pub fn new(tasks: &'a std::collections::HashMap<String, Task>) -> Self {
        Self { tasks }
    }

    /// Validate all tasks and return errors
    pub fn validate_all(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        for task in self.tasks.values() {
            errors.extend(self.validate_task(task));
        }

        errors
    }

    /// Validate a single task
    fn validate_task(&self, task: &Task) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Check dependencies exist
        for dep in task.depends() {
            if !self.tasks.contains_key(dep) {
                errors.push(ValidationError::new(
                    task.name(),
                    format!("depends on '{}' which doesn't exist", dep),
                ));
            }
        }

        // Check name matches filename
        let expected_name = Path::new(&task.file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if task.name() != expected_name {
            errors.push(ValidationError::new(
                &task.file_path,
                format!(
                    "frontmatter name '{}' doesn't match filename '{}'",
                    task.name(),
                    expected_name
                ),
            ));
        }

        // Check for circular dependencies
        if let Some(cycle) = self.detect_cycle(task.name()) {
            errors.push(ValidationError::new(
                task.name(),
                format!("circular dependency detected: {}", cycle.join(" -> ")),
            ));
        }

        errors
    }

    /// Detect circular dependency starting from a task
    pub fn detect_cycle(&self, start: &str) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        self.detect_cycle_recursive(start, &mut visited, &mut path)
    }

    fn detect_cycle_recursive(
        &self,
        current: &str,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        if path.contains(&current.to_string()) {
            // Found cycle - return the cycle path
            let cycle_start = path.iter().position(|x| x == current).unwrap();
            let mut cycle: Vec<String> = path[cycle_start..].to_vec();
            cycle.push(current.to_string());
            return Some(cycle);
        }

        if visited.contains(current) {
            return None;
        }

        visited.insert(current.to_string());
        path.push(current.to_string());

        if let Some(task) = self.tasks.get(current) {
            for dep in task.depends() {
                if let Some(cycle) = self.detect_cycle_recursive(dep, visited, path) {
                    return Some(cycle);
                }
            }
        }

        path.pop();
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Task, TaskFrontmatter};
    use std::collections::HashMap;

    fn create_test_task(name: &str, depends: Vec<&str>) -> Task {
        Task {
            frontmatter: TaskFrontmatter {
                name: name.to_string(),
                depends: depends.into_iter().map(String::from).collect(),
            },
            content: String::new(),
            file_path: format!("{}.md", name),
        }
    }

    fn create_tasks(specs: Vec<(&str, Vec<&str>)>) -> HashMap<String, Task> {
        specs
            .into_iter()
            .map(|(name, deps)| (name.to_string(), create_test_task(name, deps)))
            .collect()
    }

    #[test]
    fn test_detect_cycle_no_cycle() {
        let tasks = create_tasks(vec![("a", vec![]), ("b", vec!["a"]), ("c", vec!["b"])]);
        let validator = TaskValidator::new(&tasks);

        assert!(validator.detect_cycle("a").is_none());
        assert!(validator.detect_cycle("b").is_none());
        assert!(validator.detect_cycle("c").is_none());
    }

    #[test]
    fn test_detect_cycle_simple_cycle() {
        let tasks = create_tasks(vec![("a", vec!["b"]), ("b", vec!["a"])]);
        let validator = TaskValidator::new(&tasks);

        let cycle = validator.detect_cycle("a");
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert!(cycle.contains(&"a".to_string()));
        assert!(cycle.contains(&"b".to_string()));
    }

    #[test]
    fn test_detect_cycle_self_reference() {
        let tasks = create_tasks(vec![("a", vec!["a"])]);
        let validator = TaskValidator::new(&tasks);

        assert!(validator.detect_cycle("a").is_some());
    }

    #[test]
    fn test_detect_cycle_long_chain() {
        let tasks = create_tasks(vec![
            ("a", vec!["b"]),
            ("b", vec!["c"]),
            ("c", vec!["d"]),
            ("d", vec!["a"]),
        ]);
        let validator = TaskValidator::new(&tasks);

        assert!(validator.detect_cycle("a").is_some());
    }

    #[test]
    fn test_detect_cycle_diamond_no_cycle() {
        let tasks = create_tasks(vec![
            ("d", vec![]),
            ("b", vec!["d"]),
            ("c", vec!["d"]),
            ("a", vec!["b", "c"]),
        ]);
        let validator = TaskValidator::new(&tasks);

        assert!(validator.detect_cycle("a").is_none());
    }

    #[test]
    fn test_detect_cycle_missing_dependency() {
        let tasks = create_tasks(vec![("a", vec!["nonexistent"])]);
        let validator = TaskValidator::new(&tasks);

        // Should not panic, should return None (no cycle, just missing dep)
        assert!(validator.detect_cycle("a").is_none());
    }

    #[test]
    fn test_validate_missing_dependency() {
        let tasks = create_tasks(vec![("a", vec!["missing"])]);
        let validator = TaskValidator::new(&tasks);

        let errors = validator.validate_all();
        let dep_errors: Vec<_> = errors.iter().filter(|e| e.message.contains("depends")).collect();
        assert_eq!(dep_errors.len(), 1);
        assert!(dep_errors[0].message.contains("missing"));
    }

    #[test]
    fn test_validate_circular_dependency() {
        let tasks = create_tasks(vec![("a", vec!["b"]), ("b", vec!["a"])]);
        let validator = TaskValidator::new(&tasks);

        let errors = validator.validate_all();
        let cycle_errors: Vec<_> = errors.iter().filter(|e| e.message.contains("circular")).collect();
        assert!(!cycle_errors.is_empty());
    }
}
