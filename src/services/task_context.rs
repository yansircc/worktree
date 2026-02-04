//! Task execution context for commands.
//!
//! Encapsulates the common load → resolve → validate → execute → save pattern
//! used by most task commands.

use crate::error::Result;
use crate::models::{Instance, TaskState, TaskStatus, TaskStore, WtConfig};
use crate::services::git;

/// Task execution context - encapsulates load/resolve/save workflow.
///
/// Most commands follow this pattern:
/// 1. Load config and store
/// 2. Resolve task reference to name
/// 3. Validate task exists and check status
/// 4. Execute operations
/// 5. Save status changes
///
/// `TaskContext` encapsulates steps 1-3 and 5, letting commands focus on step 4.
pub struct TaskContext {
    /// Task store (tasks + status)
    pub store: TaskStore,
    /// Configuration
    pub config: WtConfig,
    /// Resolved task name
    pub task_name: String,
    /// Repository root path (cached)
    repo_root: Option<String>,
}

impl TaskContext {
    /// Load task context from a task reference (name or index).
    ///
    /// This performs:
    /// 1. Load config (with defaults if missing)
    /// 2. Load task store
    /// 3. Resolve task reference to actual name
    /// 4. Verify task exists (in tasks or status)
    pub fn load(task_ref: &str) -> Result<Self> {
        let config = WtConfig::load().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to load config: {}", e);
            WtConfig::default()
        });
        let store = TaskStore::load()?;
        let task_name = store.resolve_task_ref(task_ref)?;

        Ok(Self {
            store,
            config,
            task_name,
            repo_root: None,
        })
    }

    /// Load task context and ensure the task has a file definition (not scratch-only).
    pub fn load_with_task_file(task_ref: &str) -> Result<Self> {
        let ctx = Self::load(task_ref)?;
        ctx.store.ensure_exists(&ctx.task_name)?;
        Ok(ctx)
    }

    // =========================================================================
    // Accessors
    // =========================================================================

    /// Get the task name.
    pub fn name(&self) -> &str {
        &self.task_name
    }

    /// Get task status.
    pub fn status(&self) -> TaskStatus {
        self.store.status.get_status(&self.task_name)
    }

    /// Get task state (mutable).
    pub fn state_mut(&mut self) -> &mut TaskState {
        self.store.status.get_mut(&self.task_name)
    }

    /// Get task instance (if started).
    pub fn instance(&self) -> Option<&Instance> {
        self.store.status.get_instance(&self.task_name)
    }

    /// Check if this is a scratch environment.
    pub fn is_scratch(&self) -> bool {
        self.store.is_scratch(&self.task_name)
    }

    /// Get repository root (cached).
    pub fn repo_root(&mut self) -> Result<&str> {
        if self.repo_root.is_none() {
            self.repo_root = Some(git::get_repo_root()?);
        }
        Ok(self.repo_root.as_ref().unwrap())
    }

    // =========================================================================
    // Status updates
    // =========================================================================

    /// Set task status.
    pub fn set_status(&mut self, status: TaskStatus) {
        self.store.status.set_status(&self.task_name, status);
    }

    /// Save status changes.
    pub fn save_status(&self) -> Result<()> {
        self.store.status.save()
    }
}
