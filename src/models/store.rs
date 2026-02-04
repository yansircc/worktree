use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::constants::TASKS_DIR;
use crate::error::{Result, WtError};
use crate::models::{
    task_parser, task_resolver::TaskResolver, validator::TaskValidator, StatusStore, Task,
    TaskInput, TaskStatus,
};
use crate::services::multiplexer::create_multiplexer;

#[derive(Debug, Default)]
pub struct TaskStore {
    pub tasks: HashMap<String, Task>,
    pub status: StatusStore,
}

impl TaskStore {
    /// Load all tasks from .wt/tasks/*.md and status from .wt/status.json
    pub fn load() -> Result<Self> {
        let status = StatusStore::load()?;

        let dir = Path::new(TASKS_DIR);
        if !dir.exists() {
            return Ok(Self {
                tasks: HashMap::new(),
                status,
            });
        }

        let mut tasks = HashMap::new();
        let entries = fs::read_dir(dir).map_err(|e| WtError::Io {
            operation: "read directory".to_string(),
            path: TASKS_DIR.to_string(),
            message: e.to_string(),
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| WtError::Io {
                operation: "read directory entry".to_string(),
                path: TASKS_DIR.to_string(),
                message: e.to_string(),
            })?;
            let path = entry.path();
            if path.extension().map(|e| e == "md").unwrap_or(false) {
                match task_parser::parse_file(&path) {
                    Ok(task) => {
                        tasks.insert(task.name().to_string(), task);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(Self { tasks, status })
    }

    /// Get task by name
    pub fn get(&self, name: &str) -> Option<&Task> {
        self.tasks.get(name)
    }

    /// Ensure task exists, otherwise return TaskNotFound error
    pub fn ensure_exists(&self, name: &str) -> Result<&Task> {
        self.get(name)
            .ok_or_else(|| WtError::TaskNotFound(name.to_string()))
    }

    /// Resolve a task reference (name or 1-based index) to a task name.
    /// Priority: exact name match (task file or scratch) > numeric index
    pub fn resolve_task_ref(&self, task_ref: &str) -> Result<String> {
        let status_names: std::collections::HashSet<String> =
            self.status.tasks.keys().cloned().collect();
        let resolver = TaskResolver::new(&self.tasks, &status_names);
        resolver.resolve(task_ref)
    }

    /// List all tasks sorted by name
    pub fn list(&self) -> Vec<&Task> {
        let mut tasks: Vec<_> = self.tasks.values().collect();
        tasks.sort_by(|a, b| a.name().cmp(b.name()));
        tasks
    }

    /// Check if a task is a scratch environment
    pub fn is_scratch(&self, name: &str) -> bool {
        self.status
            .tasks
            .get(name)
            .and_then(|s| s.scratch)
            .unwrap_or(false)
    }

    /// Set scratch flag for a task
    pub fn set_scratch(&mut self, name: &str, scratch: bool) {
        self.status
            .tasks
            .entry(name.to_string())
            .or_default()
            .scratch = Some(scratch);
    }

    /// Check if a name exists in status.json (for scratch name collision)
    pub fn name_exists_in_status(&self, name: &str) -> bool {
        self.status.tasks.contains_key(name)
    }

    /// Check if a task should be auto-marked as Idle.
    /// Condition: status is Active but multiplexer window is closed.
    /// Returns: whether auto-mark was performed.
    pub fn auto_mark_idle_if_needed(&mut self, task_name: &str) -> Result<bool> {
        let status = self.status.get_status(task_name);
        if status != TaskStatus::Active {
            return Ok(false);
        }

        let state = match self.status.tasks.get(task_name) {
            Some(s) => s,
            None => return Ok(false),
        };

        let instance = match &state.instance {
            Some(inst) => inst,
            None => return Ok(false),
        };

        // Check if multiplexer window still exists
        let window = match &instance.window_name {
            Some(w) => w,
            None => return Ok(false),
        };
        let mux = create_multiplexer(instance.multiplexer_type());
        if mux.window_exists(&instance.session_name, window) {
            return Ok(false);
        }

        // Window closed, auto-mark as Idle
        self.status.set_status(task_name, TaskStatus::Idle);
        Ok(true)
    }

    /// Create a new task from JSON input
    pub fn create(input: &TaskInput) -> Result<PathBuf> {
        // Validate name
        task_parser::validate_name(&input.name)?;

        // Ensure directory exists
        let dir = Path::new(TASKS_DIR);
        if !dir.exists() {
            fs::create_dir_all(dir).map_err(|e| WtError::Io {
                operation: "create tasks directory".to_string(),
                path: TASKS_DIR.to_string(),
                message: e.to_string(),
            })?;
        }

        // Check if task already exists
        let file_path = dir.join(format!("{}.md", input.name));
        if file_path.exists() {
            return Err(WtError::TaskExists(input.name.clone()));
        }

        // Validate depends exist
        if !input.depends.is_empty() {
            let store = Self::load()?;
            for dep in &input.depends {
                if !store.tasks.contains_key(dep) {
                    return Err(WtError::DependencyNotFound(dep.clone()));
                }
            }
        }

        // Write file
        let markdown = input.to_markdown();
        fs::write(&file_path, &markdown).map_err(|e| WtError::Io {
            operation: "create task file".to_string(),
            path: file_path.to_string_lossy().to_string(),
            message: e.to_string(),
        })?;

        Ok(file_path)
    }

    /// Validate all tasks and return errors
    pub fn validate(&self) -> Vec<(String, String)> {
        let validator = TaskValidator::new(&self.tasks);
        validator
            .validate_all()
            .into_iter()
            .map(|e| (e.task, e.message))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Instance, TaskFrontmatter};

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

    // ==================== validate Tests ====================
    // Note: Cycle detection tests are in models/validator.rs

    #[test]
    fn test_validate_all_valid() {
        let mut store = TaskStore::default();
        store
            .tasks
            .insert("a".to_string(), create_test_task("a", vec![]));
        store
            .tasks
            .insert("b".to_string(), create_test_task("b", vec!["a"]));

        let errors = store.validate();
        // Will have name mismatch errors because file_path is "a.md" but we need full path
        // Let's filter to just dependency errors
        let dep_errors: Vec<_> = errors
            .iter()
            .filter(|(_, e)| e.contains("depends"))
            .collect();
        assert!(dep_errors.is_empty());
    }

    #[test]
    fn test_validate_missing_dependency() {
        let mut store = TaskStore::default();
        store
            .tasks
            .insert("a".to_string(), create_test_task("a", vec!["missing"]));

        let errors = store.validate();
        let dep_errors: Vec<_> = errors
            .iter()
            .filter(|(_, e)| e.contains("depends"))
            .collect();
        assert_eq!(dep_errors.len(), 1);
        assert!(dep_errors[0].1.contains("missing"));
    }

    #[test]
    fn test_validate_circular_dependency() {
        let mut store = TaskStore::default();
        store
            .tasks
            .insert("a".to_string(), create_test_task("a", vec!["b"]));
        store
            .tasks
            .insert("b".to_string(), create_test_task("b", vec!["a"]));

        let errors = store.validate();
        let cycle_errors: Vec<_> = errors
            .iter()
            .filter(|(_, e)| e.contains("circular"))
            .collect();
        assert!(!cycle_errors.is_empty());
    }

    // ==================== list Tests ====================

    #[test]
    fn test_list_sorted() {
        let mut store = TaskStore::default();
        store
            .tasks
            .insert("zebra".to_string(), create_test_task("zebra", vec![]));
        store
            .tasks
            .insert("alpha".to_string(), create_test_task("alpha", vec![]));
        store
            .tasks
            .insert("middle".to_string(), create_test_task("middle", vec![]));

        let list = store.list();
        assert_eq!(list[0].name(), "alpha");
        assert_eq!(list[1].name(), "middle");
        assert_eq!(list[2].name(), "zebra");
    }

    #[test]
    fn test_list_empty() {
        let store = TaskStore::default();
        let list = store.list();
        assert!(list.is_empty());
    }

    // ==================== Status Tests ====================

    #[test]
    fn test_store_get_status_default() {
        let store = TaskStore::default();
        assert_eq!(store.status.get_status("any"), TaskStatus::Pending);
    }

    #[test]
    fn test_store_set_and_get_status() {
        let mut store = TaskStore::default();
        store.status.set_status("test", TaskStatus::Active);
        assert_eq!(store.status.get_status("test"), TaskStatus::Active);
    }

    #[test]
    fn test_store_set_and_get_instance() {
        use crate::services::multiplexer::MultiplexerType;

        let mut store = TaskStore::default();
        let instance = Instance {
            branch: Some("wt/test".to_string()),
            worktree_path: Some("/path".to_string()),
            session_name: "wt".to_string(),
            window_name: Some("test".to_string()),
            session_id: None,
            multiplexer: MultiplexerType::Tmux,
        };
        store.status.set_instance("test", Some(instance));
        assert!(store.status.get_instance("test").is_some());
        assert_eq!(store.status.get_instance("test").unwrap().branch, Some("wt/test".to_string()));
    }

    #[test]
    fn test_store_is_scratch_default() {
        let store = TaskStore::default();
        assert!(!store.is_scratch("nonexistent"));
    }

    #[test]
    fn test_store_set_and_get_scratch() {
        let mut store = TaskStore::default();
        store.set_scratch("test", true);
        assert!(store.is_scratch("test"));

        store.set_scratch("test", false);
        assert!(!store.is_scratch("test"));
    }

    #[test]
    fn test_store_name_exists_in_status() {
        let mut store = TaskStore::default();
        assert!(!store.name_exists_in_status("test"));

        store.status.set_status("test", TaskStatus::Active);
        assert!(store.name_exists_in_status("test"));
    }

    // ==================== ensure_exists Tests ====================

    #[test]
    fn test_ensure_exists_found() {
        let mut store = TaskStore::default();
        store
            .tasks
            .insert("test".to_string(), create_test_task("test", vec![]));

        let result = store.ensure_exists("test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name(), "test");
    }

    #[test]
    fn test_ensure_exists_not_found() {
        let store = TaskStore::default();

        let result = store.ensure_exists("nonexistent");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("nonexistent"));
        assert!(err.contains("not found"));
    }

    // ==================== resolve_task_ref Tests ====================
    // Note: Detailed resolver tests are in models/task_resolver.rs

    #[test]
    fn test_resolve_task_ref_delegates_to_resolver() {
        let mut store = TaskStore::default();
        store
            .tasks
            .insert("alpha".to_string(), create_test_task("alpha", vec![]));
        store
            .tasks
            .insert("beta".to_string(), create_test_task("beta", vec![]));

        // By name
        assert_eq!(store.resolve_task_ref("alpha").unwrap(), "alpha");
        // By index
        assert_eq!(store.resolve_task_ref("1").unwrap(), "alpha");
        // Not found
        assert!(store.resolve_task_ref("nonexistent").is_err());
    }
}
