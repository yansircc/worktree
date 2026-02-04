//! Status persistence for task management.
//!
//! Provides `StatusStore` for loading and saving task runtime states.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Result, WtError};
use crate::models::status::{TaskState, TaskStatus};
use crate::models::Instance;

/// Path to the status file
pub const STATUS_FILE: &str = ".wt/status.json";

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

        serde_json::from_str(&content)
            .map_err(|e| WtError::InvalidTaskFile(format!("Invalid status.json: {}", e)))
    }

    /// Save status to .wt/status.json (atomic write)
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

        let content = serde_json::to_string_pretty(&self)
            .map_err(|e| WtError::InvalidTaskFile(format!("Failed to serialize status: {}", e)))?;

        // Atomic write: temp file + rename
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

    // =========================================================================
    // Accessors
    // =========================================================================

    /// Get state for a task (default: Pending)
    pub fn get(&self, name: &str) -> TaskState {
        self.tasks.get(name).cloned().unwrap_or_default()
    }

    /// Get mutable reference to task state, creating default if not exists
    pub fn get_mut(&mut self, name: &str) -> &mut TaskState {
        self.tasks.entry(name.to_string()).or_default()
    }

    /// Get status for a task
    pub fn get_status(&self, name: &str) -> TaskStatus {
        self.tasks
            .get(name)
            .map(|s| s.status)
            .unwrap_or_default()
    }

    /// Get instance for a task
    pub fn get_instance(&self, name: &str) -> Option<&Instance> {
        self.tasks.get(name).and_then(|s| s.instance.as_ref())
    }

    /// Set instance for a task
    pub fn set_instance(&mut self, name: &str, instance: Option<Instance>) {
        self.get_mut(name).instance = instance;
    }

    /// Set status for a task
    pub fn set_status(&mut self, name: &str, status: TaskStatus) {
        self.get_mut(name).status = status;
    }

    /// Get phase for a task
    pub fn get_phase(&self, name: &str) -> Option<&str> {
        self.tasks.get(name).and_then(|s| s.phase.as_deref())
    }

    /// Get idle reason for a task
    pub fn get_step_result(&self, name: &str) -> Option<&crate::models::status::StepResult> {
        self.tasks.get(name).and_then(|s| s.step_result.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::status::StepResult;

    #[test]
    fn test_store_default() {
        let store = StatusStore::default();
        assert!(store.tasks.is_empty());
    }

    #[test]
    fn test_store_get_default() {
        let store = StatusStore::default();
        let state = store.get("nonexistent");
        assert_eq!(state.status, TaskStatus::Pending);
    }

    #[test]
    fn test_store_get_mut() {
        let mut store = StatusStore::default();
        {
            let state = store.get_mut("test");
            state.status = TaskStatus::Active;
            state.phase = Some("developing".to_string());
        }

        let got = store.get("test");
        assert_eq!(got.status, TaskStatus::Active);
    }

    #[test]
    fn test_store_serialize() {
        let mut store = StatusStore::default();
        store.get_mut("task1").status = TaskStatus::Active;
        store.get_mut("task1").phase = Some("developing".to_string());
        store.get_mut("task2").to_idle(StepResult::Done);

        let json = serde_json::to_string(&store).unwrap();
        assert!(json.contains("task1"));
        assert!(json.contains("task2"));
        assert!(json.contains("active"));
        assert!(json.contains("idle"));
    }

    #[test]
    fn test_store_deserialize() {
        let json = r#"{
            "tasks": {
                "test": {
                    "status": "active",
                    "phase": "developing",
                    "active_since": "2026-02-03T10:30:00Z"
                }
            }
        }"#;
        let store: StatusStore = serde_json::from_str(json).unwrap();

        let state = store.get("test");
        assert_eq!(state.status, TaskStatus::Active);
        assert_eq!(state.phase, Some("developing".to_string()));
        assert!(state.active_since.is_some());
    }

    #[test]
    fn test_store_get_phase() {
        let mut store = StatusStore::default();
        store.get_mut("test").phase = Some("developing".to_string());

        assert_eq!(store.get_phase("test"), Some("developing"));
        assert_eq!(store.get_phase("nonexistent"), None);
    }
}
