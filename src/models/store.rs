use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::constants::TASKS_DIR;
use crate::error::{Result, WtError};
use crate::models::{Instance, StatusStore, Task, TaskFrontmatter, TaskInput, TaskStatus};
use crate::services::tmux;

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
            return Ok(Self { tasks: HashMap::new(), status });
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
                match Self::parse_task_file(&path) {
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

    /// Parse a single task file
    fn parse_task_file(path: &Path) -> Result<Task> {
        let content = fs::read_to_string(path).map_err(|e| WtError::Io {
            operation: "read task file".to_string(),
            path: path.to_string_lossy().to_string(),
            message: e.to_string(),
        })?;
        Self::parse_markdown(&content, path.to_string_lossy().to_string())
    }

    /// Parse markdown with frontmatter
    pub fn parse_markdown(content: &str, file_path: String) -> Result<Task> {
        let content = content.trim();
        if !content.starts_with("---") {
            return Err(WtError::InvalidTaskFile(
                "Missing frontmatter (must start with ---)".to_string(),
            ));
        }

        let rest = &content[3..];
        let end = rest.find("---").ok_or_else(|| {
            WtError::InvalidTaskFile("Missing frontmatter end (---)".to_string())
        })?;

        let yaml = &rest[..end];
        let body = rest[end + 3..].trim();

        let frontmatter: TaskFrontmatter = serde_yaml::from_str(yaml).map_err(|e| {
            WtError::InvalidTaskFile(format!("Invalid frontmatter YAML: {}", e))
        })?;

        Ok(Task {
            frontmatter,
            content: body.to_string(),
            file_path,
        })
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
        // Priority 1: exact name match in task files
        if self.tasks.contains_key(task_ref) {
            return Ok(task_ref.to_string());
        }
        // Priority 2: exact name match in status.json (scratch environments)
        if self.status.tasks.contains_key(task_ref) {
            return Ok(task_ref.to_string());
        }
        // Priority 3: try numeric index (1-based)
        if let Ok(index) = task_ref.parse::<usize>() {
            return self.get_name_by_index(index);
        }
        Err(WtError::TaskNotFound(task_ref.to_string()))
    }

    /// Get task name by 1-based index (sorted alphabetically)
    fn get_name_by_index(&self, index: usize) -> Result<String> {
        let tasks = self.list();
        let total = tasks.len();
        if index == 0 || index > total {
            return Err(WtError::InvalidTaskIndex { index, total });
        }
        Ok(tasks[index - 1].name().to_string())
    }

    /// Validate that status transition is allowed
    pub fn validate_transition(&self, name: &str, target: TaskStatus) -> Result<()> {
        let current = self.get_status(name);
        if !current.can_transition_to(&target) {
            return Err(WtError::InvalidStateTransition {
                from: current.display_name().to_string(),
                to: target.display_name().to_string(),
            });
        }
        Ok(())
    }

    /// List all tasks sorted by name
    pub fn list(&self) -> Vec<&Task> {
        let mut tasks: Vec<_> = self.tasks.values().collect();
        tasks.sort_by(|a, b| a.name().cmp(b.name()));
        tasks
    }

    // ==================== Status Accessors ====================

    /// Get status for a task (default: Pending)
    pub fn get_status(&self, name: &str) -> TaskStatus {
        self.status.get_status(name)
    }

    /// Set status for a task
    pub fn set_status(&mut self, name: &str, status: TaskStatus) {
        self.status.set_status(name, status);
    }

    /// Get instance for a task
    pub fn get_instance(&self, name: &str) -> Option<&Instance> {
        self.status.get_instance(name)
    }

    /// Set instance for a task
    pub fn set_instance(&mut self, name: &str, instance: Option<Instance>) {
        self.status.set_instance(name, instance);
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

    /// Save status to .wt/status.json
    pub fn save_status(&self) -> Result<()> {
        self.status.save()
    }

    /// Check if a task should be auto-marked as Done.
    /// Condition: status is Running but tmux window is closed.
    /// Returns: whether auto-mark was performed.
    pub fn auto_mark_done_if_needed(&mut self, task_name: &str) -> Result<bool> {
        let status = self.get_status(task_name);
        if status != TaskStatus::Running {
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

        // Check if tmux session still exists (each task has its own session)
        if tmux::session_exists(&instance.tmux_session) {
            return Ok(false);
        }

        // Session closed, auto-mark as Done
        self.set_status(task_name, TaskStatus::Done);
        Ok(true)
    }

    /// Create a new task from JSON input
    pub fn create(input: &TaskInput) -> Result<PathBuf> {
        // Validate name
        Self::validate_task_name(&input.name)?;

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
        let mut errors = Vec::new();

        for task in self.tasks.values() {
            // Check depends exist
            for dep in task.depends() {
                if !self.tasks.contains_key(dep) {
                    errors.push((
                        task.name().to_string(),
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
                errors.push((
                    task.file_path.clone(),
                    format!(
                        "frontmatter name '{}' doesn't match filename '{}'",
                        task.name(),
                        expected_name
                    ),
                ));
            }

            // Check for circular dependencies
            if let Some(cycle) = self.detect_cycle(task.name()) {
                errors.push((
                    task.name().to_string(),
                    format!("circular dependency detected: {}", cycle.join(" -> ")),
                ));
            }
        }

        errors
    }

    /// Detect circular dependency starting from a task
    fn detect_cycle(&self, start: &str) -> Option<Vec<String>> {
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

        if let Some(task) = self.get(current) {
            for dep in task.depends() {
                if let Some(cycle) = self.detect_cycle_recursive(dep, visited, path) {
                    return Some(cycle);
                }
            }
        }

        path.pop();
        None
    }

    /// Validate task name for git branch compatibility
    pub fn validate_task_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(WtError::InvalidTaskFile("name cannot be empty".to_string()));
        }
        if name.contains('/') || name.contains('\\') {
            return Err(WtError::InvalidTaskFile(
                "name cannot contain path separators".to_string(),
            ));
        }
        if name.contains(' ') || name.contains('\t') {
            return Err(WtError::InvalidTaskFile(
                "name cannot contain whitespace (invalid for git branch)".to_string(),
            ));
        }
        let invalid_chars = ['~', '^', ':', '?', '*', '[', '\\', '@', '{'];
        for c in invalid_chars {
            if name.contains(c) {
                return Err(WtError::InvalidTaskFile(format!(
                    "name cannot contain '{}' (invalid for git branch)",
                    c
                )));
            }
        }
        if name.starts_with('-') || name.starts_with('.') {
            return Err(WtError::InvalidTaskFile(
                "name cannot start with '-' or '.' (invalid for git branch)".to_string(),
            ));
        }
        if name.ends_with('.') || name.ends_with(".lock") {
            return Err(WtError::InvalidTaskFile(
                "name cannot end with '.' or '.lock' (invalid for git branch)".to_string(),
            ));
        }
        if name.contains("..") {
            return Err(WtError::InvalidTaskFile(
                "name cannot contain '..' (invalid for git branch)".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== parse_markdown Tests ====================

    #[test]
    fn test_parse_markdown_simple() {
        let content = "---\nname: test\n---\n\nDescription here";
        let task = TaskStore::parse_markdown(content, "test.md".to_string()).unwrap();

        assert_eq!(task.name(), "test");
        assert_eq!(task.content, "Description here");
    }

    #[test]
    fn test_parse_markdown_with_depends() {
        let content = "---\nname: api\ndepends:\n  - auth\n  - db\n---\n\nBuild API";
        let task = TaskStore::parse_markdown(content, "api.md".to_string()).unwrap();

        assert_eq!(task.name(), "api");
        assert_eq!(task.depends(), &["auth".to_string(), "db".to_string()]);
    }

    #[test]
    fn test_parse_markdown_multiline_content() {
        let content = "---\nname: test\n---\n\nLine 1\n\nLine 2\n\n- bullet";
        let task = TaskStore::parse_markdown(content, "test.md".to_string()).unwrap();

        assert_eq!(task.content, "Line 1\n\nLine 2\n\n- bullet");
    }

    #[test]
    fn test_parse_markdown_empty_content() {
        let content = "---\nname: test\n---\n";
        let task = TaskStore::parse_markdown(content, "test.md".to_string()).unwrap();

        assert_eq!(task.content, "");
    }

    #[test]
    fn test_parse_markdown_unicode() {
        let content = "---\nname: 任务\n---\n\n中文描述 🚀";
        let task = TaskStore::parse_markdown(content, "test.md".to_string()).unwrap();

        assert_eq!(task.name(), "任务");
        assert_eq!(task.content, "中文描述 🚀");
    }

    #[test]
    fn test_parse_markdown_missing_frontmatter_start() {
        let content = "name: test\n---\nContent";
        let result = TaskStore::parse_markdown(content, "test.md".to_string());

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Missing frontmatter"));
    }

    #[test]
    fn test_parse_markdown_missing_frontmatter_end() {
        let content = "---\nname: test\nContent without end";
        let result = TaskStore::parse_markdown(content, "test.md".to_string());

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Missing frontmatter end"));
    }

    #[test]
    fn test_parse_markdown_invalid_yaml() {
        let content = "---\nname: [invalid yaml\n---\nContent";
        let result = TaskStore::parse_markdown(content, "test.md".to_string());

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid frontmatter YAML"));
    }

    #[test]
    fn test_parse_markdown_missing_name() {
        let content = "---\ndepends: []\n---\nContent";
        let result = TaskStore::parse_markdown(content, "test.md".to_string());

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_markdown_whitespace_trimmed() {
        let content = "  \n---\nname: test\n---\n\nContent  \n  ";
        let task = TaskStore::parse_markdown(content, "test.md".to_string()).unwrap();

        assert_eq!(task.name(), "test");
    }

    // ==================== validate_task_name Tests ====================

    #[test]
    fn test_validate_name_valid() {
        assert!(TaskStore::validate_task_name("auth").is_ok());
        assert!(TaskStore::validate_task_name("my-task").is_ok());
        assert!(TaskStore::validate_task_name("task_123").is_ok());
        assert!(TaskStore::validate_task_name("CamelCase").is_ok());
        assert!(TaskStore::validate_task_name("a").is_ok());
        assert!(TaskStore::validate_task_name("task.name").is_ok()); // single dot ok
    }

    #[test]
    fn test_validate_name_empty() {
        let result = TaskStore::validate_task_name("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_name_path_separators() {
        assert!(TaskStore::validate_task_name("path/to").is_err());
        assert!(TaskStore::validate_task_name("path\\to").is_err());
    }

    #[test]
    fn test_validate_name_whitespace() {
        assert!(TaskStore::validate_task_name("has space").is_err());
        assert!(TaskStore::validate_task_name("has\ttab").is_err());
        assert!(TaskStore::validate_task_name(" leading").is_err());
        assert!(TaskStore::validate_task_name("trailing ").is_err());
    }

    #[test]
    fn test_validate_name_git_invalid_chars() {
        let invalid = ['~', '^', ':', '?', '*', '[', '@', '{'];
        for c in invalid {
            let name = format!("task{}name", c);
            let result = TaskStore::validate_task_name(&name);
            assert!(result.is_err(), "Should reject char: {}", c);
        }
    }

    #[test]
    fn test_validate_name_invalid_start() {
        assert!(TaskStore::validate_task_name("-dash").is_err());
        assert!(TaskStore::validate_task_name(".hidden").is_err());
    }

    #[test]
    fn test_validate_name_invalid_end() {
        assert!(TaskStore::validate_task_name("name.").is_err());
        assert!(TaskStore::validate_task_name("name.lock").is_err());
    }

    #[test]
    fn test_validate_name_double_dot() {
        assert!(TaskStore::validate_task_name("a..b").is_err());
        assert!(TaskStore::validate_task_name("..").is_err());
    }

    // ==================== Cycle Detection Tests ====================

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

    #[test]
    fn test_detect_cycle_no_cycle() {
        let mut store = TaskStore::default();
        store.tasks.insert("a".to_string(), create_test_task("a", vec![]));
        store.tasks.insert("b".to_string(), create_test_task("b", vec!["a"]));
        store.tasks.insert("c".to_string(), create_test_task("c", vec!["b"]));

        assert!(store.detect_cycle("a").is_none());
        assert!(store.detect_cycle("b").is_none());
        assert!(store.detect_cycle("c").is_none());
    }

    #[test]
    fn test_detect_cycle_simple_cycle() {
        let mut store = TaskStore::default();
        store.tasks.insert("a".to_string(), create_test_task("a", vec!["b"]));
        store.tasks.insert("b".to_string(), create_test_task("b", vec!["a"]));

        let cycle = store.detect_cycle("a");
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert!(cycle.contains(&"a".to_string()));
        assert!(cycle.contains(&"b".to_string()));
    }

    #[test]
    fn test_detect_cycle_self_reference() {
        let mut store = TaskStore::default();
        store.tasks.insert("a".to_string(), create_test_task("a", vec!["a"]));

        let cycle = store.detect_cycle("a");
        assert!(cycle.is_some());
    }

    #[test]
    fn test_detect_cycle_long_chain() {
        let mut store = TaskStore::default();
        store.tasks.insert("a".to_string(), create_test_task("a", vec!["b"]));
        store.tasks.insert("b".to_string(), create_test_task("b", vec!["c"]));
        store.tasks.insert("c".to_string(), create_test_task("c", vec!["d"]));
        store.tasks.insert("d".to_string(), create_test_task("d", vec!["a"]));

        let cycle = store.detect_cycle("a");
        assert!(cycle.is_some());
    }

    #[test]
    fn test_detect_cycle_diamond_no_cycle() {
        // a -> b, a -> c, b -> d, c -> d (diamond, no cycle)
        let mut store = TaskStore::default();
        store.tasks.insert("d".to_string(), create_test_task("d", vec![]));
        store.tasks.insert("b".to_string(), create_test_task("b", vec!["d"]));
        store.tasks.insert("c".to_string(), create_test_task("c", vec!["d"]));
        store.tasks.insert("a".to_string(), create_test_task("a", vec!["b", "c"]));

        assert!(store.detect_cycle("a").is_none());
    }

    #[test]
    fn test_detect_cycle_missing_dependency() {
        let mut store = TaskStore::default();
        store.tasks.insert("a".to_string(), create_test_task("a", vec!["nonexistent"]));

        // Should not panic, should return None (no cycle, just missing dep)
        assert!(store.detect_cycle("a").is_none());
    }

    // ==================== validate Tests ====================

    #[test]
    fn test_validate_all_valid() {
        let mut store = TaskStore::default();
        store.tasks.insert("a".to_string(), create_test_task("a", vec![]));
        store.tasks.insert("b".to_string(), create_test_task("b", vec!["a"]));

        let errors = store.validate();
        // Will have name mismatch errors because file_path is "a.md" but we need full path
        // Let's filter to just dependency errors
        let dep_errors: Vec<_> = errors.iter().filter(|(_, e)| e.contains("depends")).collect();
        assert!(dep_errors.is_empty());
    }

    #[test]
    fn test_validate_missing_dependency() {
        let mut store = TaskStore::default();
        store.tasks.insert("a".to_string(), create_test_task("a", vec!["missing"]));

        let errors = store.validate();
        let dep_errors: Vec<_> = errors.iter().filter(|(_, e)| e.contains("depends")).collect();
        assert_eq!(dep_errors.len(), 1);
        assert!(dep_errors[0].1.contains("missing"));
    }

    #[test]
    fn test_validate_circular_dependency() {
        let mut store = TaskStore::default();
        store.tasks.insert("a".to_string(), create_test_task("a", vec!["b"]));
        store.tasks.insert("b".to_string(), create_test_task("b", vec!["a"]));

        let errors = store.validate();
        let cycle_errors: Vec<_> = errors.iter().filter(|(_, e)| e.contains("circular")).collect();
        assert!(!cycle_errors.is_empty());
    }

    // ==================== list Tests ====================

    #[test]
    fn test_list_sorted() {
        let mut store = TaskStore::default();
        store.tasks.insert("zebra".to_string(), create_test_task("zebra", vec![]));
        store.tasks.insert("alpha".to_string(), create_test_task("alpha", vec![]));
        store.tasks.insert("middle".to_string(), create_test_task("middle", vec![]));

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
        assert_eq!(store.get_status("any"), TaskStatus::Pending);
    }

    #[test]
    fn test_store_set_and_get_status() {
        let mut store = TaskStore::default();
        store.set_status("test", TaskStatus::Running);
        assert_eq!(store.get_status("test"), TaskStatus::Running);
    }

    #[test]
    fn test_store_set_and_get_instance() {
        let mut store = TaskStore::default();
        let instance = Instance {
            branch: "wt/test".to_string(),
            worktree_path: "/path".to_string(),
            tmux_session: "wt".to_string(),
            tmux_window: "test".to_string(),
            session_id: None,
        };
        store.set_instance("test", Some(instance));
        assert!(store.get_instance("test").is_some());
        assert_eq!(store.get_instance("test").unwrap().branch, "wt/test");
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

        store.set_status("test", TaskStatus::Running);
        assert!(store.name_exists_in_status("test"));
    }

    // ==================== ensure_exists Tests ====================

    #[test]
    fn test_ensure_exists_found() {
        let mut store = TaskStore::default();
        store.tasks.insert("test".to_string(), create_test_task("test", vec![]));

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

    // ==================== validate_transition Tests ====================

    #[test]
    fn test_validate_transition_valid() {
        let mut store = TaskStore::default();
        store.set_status("test", TaskStatus::Running);

        let result = store.validate_transition("test", TaskStatus::Done);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_transition_invalid() {
        let store = TaskStore::default();
        // Default is Pending, cannot transition to Done directly

        let result = store.validate_transition("test", TaskStatus::Done);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("pending"));
        assert!(err.contains("done"));
    }

    // ==================== resolve_task_ref Tests ====================

    #[test]
    fn test_resolve_task_ref_by_name() {
        let mut store = TaskStore::default();
        store.tasks.insert("alpha".to_string(), create_test_task("alpha", vec![]));
        store.tasks.insert("beta".to_string(), create_test_task("beta", vec![]));

        let result = store.resolve_task_ref("alpha");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "alpha");
    }

    #[test]
    fn test_resolve_task_ref_by_index() {
        let mut store = TaskStore::default();
        store.tasks.insert("alpha".to_string(), create_test_task("alpha", vec![]));
        store.tasks.insert("beta".to_string(), create_test_task("beta", vec![]));
        store.tasks.insert("gamma".to_string(), create_test_task("gamma", vec![]));

        // Tasks are sorted alphabetically: alpha=1, beta=2, gamma=3
        assert_eq!(store.resolve_task_ref("1").unwrap(), "alpha");
        assert_eq!(store.resolve_task_ref("2").unwrap(), "beta");
        assert_eq!(store.resolve_task_ref("3").unwrap(), "gamma");
    }

    #[test]
    fn test_resolve_task_ref_name_priority_over_index() {
        let mut store = TaskStore::default();
        // Create a task named "2" - name should take priority over index
        store.tasks.insert("1".to_string(), create_test_task("1", vec![]));
        store.tasks.insert("2".to_string(), create_test_task("2", vec![]));
        store.tasks.insert("alpha".to_string(), create_test_task("alpha", vec![]));

        // "2" matches task name, not index 2 (which would be "2" anyway in this case)
        // But let's verify "1" works - sorted: 1, 2, alpha
        // Index 1 = "1", Index 2 = "2", Index 3 = "alpha"
        // Name "1" exists, so it should match name first
        let result = store.resolve_task_ref("1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "1");
    }

    #[test]
    fn test_resolve_task_ref_index_zero_error() {
        let mut store = TaskStore::default();
        store.tasks.insert("alpha".to_string(), create_test_task("alpha", vec![]));

        let result = store.resolve_task_ref("0");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid task index 0"));
        assert!(err.contains("valid range is 1-1"));
    }

    #[test]
    fn test_resolve_task_ref_index_out_of_range() {
        let mut store = TaskStore::default();
        store.tasks.insert("alpha".to_string(), create_test_task("alpha", vec![]));
        store.tasks.insert("beta".to_string(), create_test_task("beta", vec![]));

        let result = store.resolve_task_ref("99");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid task index 99"));
        assert!(err.contains("valid range is 1-2"));
    }

    #[test]
    fn test_resolve_task_ref_not_found() {
        let mut store = TaskStore::default();
        store.tasks.insert("alpha".to_string(), create_test_task("alpha", vec![]));

        let result = store.resolve_task_ref("nonexistent");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("nonexistent"));
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_resolve_task_ref_empty_store() {
        let store = TaskStore::default();

        let result = store.resolve_task_ref("1");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("valid range is 1-0"));
    }
}
