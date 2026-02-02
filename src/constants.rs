//! Centralized constants for path and naming conventions.

/// Task markdown files directory
pub const TASKS_DIR: &str = ".wt/tasks";

/// Configuration file name
pub const CONFIG_FILE: &str = ".wt/config.yaml";

/// Default worktree directory
pub const DEFAULT_WORKTREE_DIR: &str = ".wt/worktrees";

/// Default session name for multiplexer
pub const DEFAULT_SESSION_NAME: &str = "wt";

/// Branch name prefix for worktree tasks
pub const BRANCH_PREFIX: &str = "wt/";

/// Status file for runtime state
pub const STATUS_FILE: &str = ".wt/status.json";

/// Logs directory for debug output
pub const LOGS_DIR: &str = ".wt/logs";

/// Backups directory for reset command
pub const BACKUPS_DIR: &str = ".wt/backups";

/// Idle threshold in seconds (for status command)
pub const IDLE_THRESHOLD_SECS: u64 = 120;

/// Generate branch name from task name and session_id
/// Format: wt/{task_name}-{session_id_prefix}
pub fn branch_name(task_name: &str, session_id: &str) -> String {
    let prefix = &session_id[..4.min(session_id.len())];
    format!("{}{}-{}", BRANCH_PREFIX, task_name, prefix)
}

/// Generate glob pattern for finding task-related branches
/// Example: task_name = "auth" → "wt/auth-*"
pub fn branch_pattern(task_name: &str) -> String {
    format!("{}{}-*", BRANCH_PREFIX, task_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_name() {
        assert_eq!(branch_name("auth", "3e20cef2"), "wt/auth-3e20");
        assert_eq!(branch_name("feature-x", "a1b2c3d4"), "wt/feature-x-a1b2");
    }

    #[test]
    fn test_branch_name_short_session_id() {
        assert_eq!(branch_name("auth", "ab"), "wt/auth-ab");
        assert_eq!(branch_name("auth", ""), "wt/auth-");
    }

    #[test]
    fn test_branch_pattern() {
        assert_eq!(branch_pattern("auth"), "wt/auth-*");
        assert_eq!(branch_pattern("feature-x"), "wt/feature-x-*");
    }
}
