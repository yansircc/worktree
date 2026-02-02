use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::constants::STATUS_FILE;
use crate::error::{Result, WtError};
use crate::models::{Instance, TaskStatus};

/// Runtime state for a single task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskState {
    pub status: TaskStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance: Option<Instance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scratch: Option<bool>,
}

impl Default for TaskState {
    fn default() -> Self {
        Self {
            status: TaskStatus::Pending,
            instance: None,
            scratch: None,
        }
    }
}

/// Store for all task runtime states
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StatusStore {
    pub tasks: HashMap<String, TaskState>,
}

impl StatusStore {
    /// Load status from .wt/status.json
    pub fn load() -> Result<Self> {
        let path = Path::new(STATUS_FILE);
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path).map_err(|e| WtError::Io {
            operation: "read status file".to_string(),
            path: STATUS_FILE.to_string(),
            message: e.to_string(),
        })?;

        serde_json::from_str(&content).map_err(|e| {
            WtError::InvalidTaskFile(format!("Invalid status.json: {}", e))
        })
    }

    /// Save status to .wt/status.json (atomic write via temp file + rename)
    pub fn save(&self) -> Result<()> {
        let path = Path::new(STATUS_FILE);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| WtError::Io {
                    operation: "create status directory".to_string(),
                    path: parent.to_string_lossy().to_string(),
                    message: e.to_string(),
                })?;
            }
        }

        let content = serde_json::to_string_pretty(&self).map_err(|e| {
            WtError::InvalidTaskFile(format!("Failed to serialize status: {}", e))
        })?;

        // Atomic write: write to temp file, then rename
        let temp_path = format!("{}.tmp", STATUS_FILE);
        fs::write(&temp_path, &content).map_err(|e| WtError::Io {
            operation: "write temp status file".to_string(),
            path: temp_path.clone(),
            message: e.to_string(),
        })?;

        fs::rename(&temp_path, path).map_err(|e| WtError::Io {
            operation: "rename status file".to_string(),
            path: STATUS_FILE.to_string(),
            message: e.to_string(),
        })?;

        Ok(())
    }

    /// Get status for a task (default: Pending)
    pub fn get_status(&self, name: &str) -> TaskStatus {
        self.tasks
            .get(name)
            .map(|s| s.status.clone())
            .unwrap_or_default()
    }

    /// Set status for a task
    pub fn set_status(&mut self, name: &str, status: TaskStatus) {
        self.tasks
            .entry(name.to_string())
            .or_default()
            .status = status;
    }

    /// Get instance for a task
    pub fn get_instance(&self, name: &str) -> Option<&Instance> {
        self.tasks.get(name).and_then(|s| s.instance.as_ref())
    }

    /// Set instance for a task
    pub fn set_instance(&mut self, name: &str, instance: Option<Instance>) {
        self.tasks
            .entry(name.to_string())
            .or_default()
            .instance = instance;
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_state_default() {
        let state = TaskState::default();
        assert_eq!(state.status, TaskStatus::Pending);
        assert!(state.instance.is_none());
        assert!(state.scratch.is_none());
    }

    #[test]
    fn test_task_state_with_scratch() {
        let json = r#"{"status":"running","scratch":true}"#;
        let state: TaskState = serde_json::from_str(json).unwrap();
        assert_eq!(state.status, TaskStatus::Running);
        assert_eq!(state.scratch, Some(true));
    }

    #[test]
    fn test_task_state_scratch_serialization() {
        let mut state = TaskState::default();
        state.status = TaskStatus::Running;
        state.scratch = Some(true);

        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("\"scratch\":true"));

        // Without scratch, field should be omitted
        state.scratch = None;
        let json = serde_json::to_string(&state).unwrap();
        assert!(!json.contains("scratch"));
    }

    #[test]
    fn test_task_state_backward_compatible() {
        // Old JSON without scratch field should still deserialize
        let json = r#"{"status":"running"}"#;
        let state: TaskState = serde_json::from_str(json).unwrap();
        assert_eq!(state.status, TaskStatus::Running);
        assert!(state.scratch.is_none());
    }

    #[test]
    fn test_status_store_get_status_default() {
        let store = StatusStore::default();
        assert_eq!(store.get_status("nonexistent"), TaskStatus::Pending);
    }

    #[test]
    fn test_status_store_set_and_get_status() {
        let mut store = StatusStore::default();
        store.set_status("test", TaskStatus::Running);
        assert_eq!(store.get_status("test"), TaskStatus::Running);
    }

    #[test]
    fn test_status_store_set_and_get_instance() {
        use crate::services::multiplexer::MultiplexerType;

        let mut store = StatusStore::default();
        let instance = Instance {
            branch: "wt/test".to_string(),
            worktree_path: "/path".to_string(),
            session_name: "wt".to_string(),
            window_name: "test".to_string(),
            session_id: None,
            multiplexer: MultiplexerType::Tmux,
        };
        store.set_instance("test", Some(instance.clone()));

        let got = store.get_instance("test").unwrap();
        assert_eq!(got.branch, "wt/test");
    }

    #[test]
    fn test_status_store_serialize() {
        let mut store = StatusStore::default();
        store.set_status("task1", TaskStatus::Running);
        store.set_status("task2", TaskStatus::Done);

        let json = serde_json::to_string(&store).unwrap();
        assert!(json.contains("task1"));
        assert!(json.contains("running"));
    }

    #[test]
    fn test_status_store_deserialize() {
        let json = r#"{"tasks":{"test":{"status":"running"}}}"#;
        let store: StatusStore = serde_json::from_str(json).unwrap();
        assert_eq!(store.get_status("test"), TaskStatus::Running);
    }
}
