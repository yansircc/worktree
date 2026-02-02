//! Execution context for hooks v2.
//!
//! Provides variable expansion and environment setup for hook execution.

use std::collections::HashMap;

/// Execution context for hook steps.
///
/// Contains all variables available for expansion in hook scripts and prompts.
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Task name
    pub task: String,
    /// Branch name
    pub branch: String,
    /// Worktree path
    pub worktree: String,
    /// Repository root path
    pub repo_root: String,
    /// Multiplexer session name
    pub session: String,
    /// Multiplexer window name
    pub window: String,
    /// Current phase (developing/reviewing/merging)
    pub phase: String,
    /// Additional custom variables
    pub extra: HashMap<String, String>,
}

impl ExecutionContext {
    /// Create a new execution context with required fields.
    pub fn new(task: &str, branch: &str, worktree: &str, repo_root: &str) -> Self {
        Self {
            task: task.to_string(),
            branch: branch.to_string(),
            worktree: worktree.to_string(),
            repo_root: repo_root.to_string(),
            session: String::new(),
            window: String::new(),
            phase: String::new(),
            extra: HashMap::new(),
        }
    }

    /// Set session name.
    pub fn with_session(mut self, session: &str) -> Self {
        self.session = session.to_string();
        self
    }

    /// Set window name.
    pub fn with_window(mut self, window: &str) -> Self {
        self.window = window.to_string();
        self
    }

    /// Set current phase.
    pub fn with_phase(mut self, phase: &str) -> Self {
        self.phase = phase.to_string();
        self
    }

    /// Add a custom variable.
    pub fn with_var(mut self, key: &str, value: &str) -> Self {
        self.extra.insert(key.to_string(), value.to_string());
        self
    }

    // =========================================================================
    // Backward compatibility methods (for old HookContext API)
    // =========================================================================

    /// Set status (backward compatibility).
    pub fn with_status(mut self, status: &str) -> Self {
        self.extra.insert("status".to_string(), status.to_string());
        self
    }

    /// Set previous status (backward compatibility).
    pub fn with_prev_status(mut self, prev: &str) -> Self {
        self.extra.insert("prev_status".to_string(), prev.to_string());
        self
    }

    /// Set backup directory (backward compatibility).
    pub fn with_backup_dir(mut self, dir: &str) -> Self {
        self.extra.insert("backup_dir".to_string(), dir.to_string());
        self
    }

    /// Set timestamp (backward compatibility).
    pub fn with_timestamp(mut self, ts: &str) -> Self {
        self.extra.insert("timestamp".to_string(), ts.to_string());
        self
    }

    /// Expand variables in a string.
    ///
    /// Replaces `${variable}` patterns with their values.
    /// Unknown variables are left unchanged.
    pub fn expand(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Built-in variables
        result = result.replace("${task}", &self.task);
        result = result.replace("${branch}", &self.branch);
        result = result.replace("${worktree}", &self.worktree);
        result = result.replace("${repo_root}", &self.repo_root);
        result = result.replace("${session}", &self.session);
        result = result.replace("${window}", &self.window);
        result = result.replace("${phase}", &self.phase);

        // Custom variables
        for (key, value) in &self.extra {
            result = result.replace(&format!("${{{}}}", key), value);
        }

        result
    }

    /// Convert context to environment variables.
    ///
    /// Returns a map of WT_* environment variables.
    pub fn to_env_vars(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        vars.insert("WT_TASK".to_string(), self.task.clone());
        vars.insert("WT_BRANCH".to_string(), self.branch.clone());
        vars.insert("WT_WORKTREE".to_string(), self.worktree.clone());
        vars.insert("WT_REPO_ROOT".to_string(), self.repo_root.clone());
        vars.insert("WT_SESSION".to_string(), self.session.clone());
        vars.insert("WT_WINDOW".to_string(), self.window.clone());
        vars.insert("WT_PHASE".to_string(), self.phase.clone());

        for (key, value) in &self.extra {
            vars.insert(format!("WT_{}", key.to_uppercase()), value.clone());
        }

        vars
    }

    /// Get working directory for command execution.
    ///
    /// Returns worktree path if it exists, otherwise repo_root.
    pub fn working_dir(&self) -> &str {
        if !self.worktree.is_empty() && std::path::Path::new(&self.worktree).exists() {
            &self.worktree
        } else if !self.repo_root.is_empty() {
            &self.repo_root
        } else {
            "."
        }
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            task: String::new(),
            branch: String::new(),
            worktree: String::new(),
            repo_root: String::new(),
            session: String::new(),
            window: String::new(),
            phase: String::new(),
            extra: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_new() {
        let ctx = ExecutionContext::new("auth", "wt/auth", "/work/auth", "/repo");
        assert_eq!(ctx.task, "auth");
        assert_eq!(ctx.branch, "wt/auth");
        assert_eq!(ctx.worktree, "/work/auth");
        assert_eq!(ctx.repo_root, "/repo");
    }

    #[test]
    fn test_context_builder() {
        let ctx = ExecutionContext::new("auth", "wt/auth", "/work/auth", "/repo")
            .with_session("wt")
            .with_window("auth")
            .with_phase("developing")
            .with_var("custom", "value");

        assert_eq!(ctx.session, "wt");
        assert_eq!(ctx.window, "auth");
        assert_eq!(ctx.phase, "developing");
        assert_eq!(ctx.extra.get("custom"), Some(&"value".to_string()));
    }

    #[test]
    fn test_expand_variables() {
        let ctx = ExecutionContext::new("auth", "wt/auth", "/work/auth", "/repo")
            .with_session("wt")
            .with_phase("developing");

        assert_eq!(ctx.expand("task: ${task}"), "task: auth");
        assert_eq!(ctx.expand("@.wt/tasks/${task}.md"), "@.wt/tasks/auth.md");
        assert_eq!(
            ctx.expand("${branch} in ${worktree}"),
            "wt/auth in /work/auth"
        );
        assert_eq!(ctx.expand("phase: ${phase}"), "phase: developing");
    }

    #[test]
    fn test_expand_unknown_preserved() {
        let ctx = ExecutionContext::new("auth", "wt/auth", "/work/auth", "/repo");
        assert_eq!(ctx.expand("${unknown}"), "${unknown}");
    }

    #[test]
    fn test_expand_custom_var() {
        let ctx = ExecutionContext::new("auth", "wt/auth", "/work/auth", "/repo")
            .with_var("model", "opus");
        assert_eq!(ctx.expand("--model ${model}"), "--model opus");
    }

    #[test]
    fn test_to_env_vars() {
        let ctx = ExecutionContext::new("auth", "wt/auth", "/work/auth", "/repo")
            .with_session("wt")
            .with_var("custom", "val");

        let vars = ctx.to_env_vars();
        assert_eq!(vars.get("WT_TASK"), Some(&"auth".to_string()));
        assert_eq!(vars.get("WT_BRANCH"), Some(&"wt/auth".to_string()));
        assert_eq!(vars.get("WT_SESSION"), Some(&"wt".to_string()));
        assert_eq!(vars.get("WT_CUSTOM"), Some(&"val".to_string()));
    }

    #[test]
    fn test_working_dir() {
        // Empty context
        let ctx = ExecutionContext::default();
        assert_eq!(ctx.working_dir(), ".");

        // With repo_root only
        let ctx = ExecutionContext::new("", "", "", "/repo");
        assert_eq!(ctx.working_dir(), "/repo");
    }
}
