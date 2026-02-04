//! Centralized constants for path and naming conventions.

/// Task markdown files directory
pub const TASKS_DIR: &str = ".wt/tasks";

/// Default session name for multiplexer
pub const DEFAULT_SESSION_NAME: &str = "wt";

/// Branch name prefix for worktree tasks
pub const BRANCH_PREFIX: &str = "wt/";

/// Logs directory for debug output
pub const LOGS_DIR: &str = ".wt/logs";

/// Backups directory for reset command
pub const BACKUPS_DIR: &str = ".wt/backups";

/// Idle threshold in seconds (for status command)
pub const IDLE_THRESHOLD_SECS: u64 = 120;

/// Generate branch name for a task (deterministic naming)
/// Example: task_name = "auth" → "wt/auth"
pub fn branch_name(task_name: &str) -> String {
    format!("{}{}", BRANCH_PREFIX, task_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_name() {
        assert_eq!(branch_name("auth"), "wt/auth");
        assert_eq!(branch_name("feature-x"), "wt/feature-x");
    }
}
