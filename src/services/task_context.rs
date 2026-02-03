//! Task execution context for commands.
//!
//! Encapsulates the common load → resolve → validate → execute → save pattern
//! used by most task commands.

use std::path::Path;

use crate::constants::BACKUPS_DIR;
use crate::error::{Result, WtError};
use crate::models::{Instance, TaskState, TaskStatus, TaskStore, WtConfig};
use crate::services::{executor::ExecutionContext, git};

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
        let config = WtConfig::load().unwrap_or_default();
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
        self.store.get_status(&self.task_name)
    }

    /// Get task state (mutable).
    pub fn state_mut(&mut self) -> &mut TaskState {
        self.store.status.get_mut(&self.task_name)
    }

    /// Get task instance (if started).
    pub fn instance(&self) -> Option<&Instance> {
        self.store.get_instance(&self.task_name)
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
    // Validation helpers
    // =========================================================================

    /// Ensure this is not a scratch environment.
    pub fn require_not_scratch(&self, action: &str) -> Result<()> {
        if self.is_scratch() {
            return Err(WtError::InvalidInput(format!(
                "Scratch environment '{}' cannot be {}. Use 'wt delete {}' to clean up.",
                self.task_name, action, self.task_name
            )));
        }
        Ok(())
    }

    /// Require task status to be one of the expected values.
    pub fn require_status(&self, expected: &[TaskStatus], action: &str) -> Result<()> {
        let current = self.status();
        if !expected.contains(&current) {
            let expected_str = expected
                .iter()
                .map(|s| s.display_name())
                .collect::<Vec<_>>()
                .join(" or ");
            return Err(WtError::InvalidInput(format!(
                "Task '{}' is {} (expected {}). Cannot {}.",
                self.task_name,
                current.display_name(),
                expected_str,
                action
            )));
        }
        Ok(())
    }

    /// Require instance to exist.
    pub fn require_instance(&self) -> Result<&Instance> {
        self.instance()
            .ok_or_else(|| WtError::TaskNotStarted(self.task_name.clone()))
    }

    /// Require worktree to exist.
    pub fn require_worktree(&self) -> Result<&str> {
        let instance = self.require_instance()?;
        let path = &instance.worktree_path;
        if !Path::new(path).exists() {
            return Err(WtError::WorktreeNotFound(self.task_name.clone()));
        }
        Ok(path)
    }

    /// Validate status transition is allowed.
    pub fn validate_transition(&self, target: TaskStatus) -> Result<()> {
        self.store.validate_transition(&self.task_name, target)
    }

    // =========================================================================
    // Status updates
    // =========================================================================

    /// Set task status.
    pub fn set_status(&mut self, status: TaskStatus) {
        self.store.set_status(&self.task_name, status);
    }

    /// Save status changes.
    pub fn save_status(&self) -> Result<()> {
        self.store.save_status()
    }

    // =========================================================================
    // Hook context building
    // =========================================================================

    /// Build an ExecutionContext for hook execution.
    ///
    /// Automatically fills in task, branch, worktree, repo_root from instance.
    pub fn build_hook_context(&mut self) -> Result<ExecutionContext> {
        let repo_root = self.repo_root()?.to_string();

        let ctx = if let Some(instance) = self.instance() {
            ExecutionContext::new(
                &self.task_name,
                &instance.branch,
                &instance.worktree_path,
                &repo_root,
            )
            .with_session(&instance.session_name)
            .with_window(&instance.window_name)
        } else {
            ExecutionContext::new(&self.task_name, "", "", &repo_root)
        };

        Ok(ctx.with_backup_dir(BACKUPS_DIR))
    }
}
