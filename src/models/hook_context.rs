use std::collections::HashMap;

/// Context for hook script variable expansion
///
/// Contains all variables available in hook scripts.
/// Variables are referenced as `${variable_name}` in scripts.
#[derive(Debug, Clone, Default)]
pub struct HookContext {
    pub task: String,
    pub branch: String,
    pub worktree: String,
    pub repo_root: String,
    pub session: String,
    pub window: String,
    pub status: String,
    pub prev_status: Option<String>,
    pub timestamp: String,
    pub backup_dir: String,
}

impl HookContext {
    /// Create a new HookContext with required fields
    pub fn new(task: &str, branch: &str, worktree: &str, repo_root: &str) -> Self {
        Self {
            task: task.to_string(),
            branch: branch.to_string(),
            worktree: worktree.to_string(),
            repo_root: repo_root.to_string(),
            session: String::new(),
            window: String::new(),
            status: String::new(),
            prev_status: None,
            timestamp: String::new(),
            backup_dir: String::new(),
        }
    }

    /// Set multiplexer session name
    pub fn with_session(mut self, session: &str) -> Self {
        self.session = session.to_string();
        self
    }

    /// Set multiplexer window name
    pub fn with_window(mut self, window: &str) -> Self {
        self.window = window.to_string();
        self
    }

    /// Set current task status
    pub fn with_status(mut self, status: &str) -> Self {
        self.status = status.to_string();
        self
    }

    /// Set previous task status (for transitions)
    pub fn with_prev_status(mut self, prev_status: &str) -> Self {
        self.prev_status = Some(prev_status.to_string());
        self
    }

    /// Set timestamp
    pub fn with_timestamp(mut self, timestamp: &str) -> Self {
        self.timestamp = timestamp.to_string();
        self
    }

    /// Set backup directory
    pub fn with_backup_dir(mut self, backup_dir: &str) -> Self {
        self.backup_dir = backup_dir.to_string();
        self
    }

    /// Convert context to a HashMap of variable names to values
    pub fn to_env_vars(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        vars.insert("task".to_string(), self.task.clone());
        vars.insert("branch".to_string(), self.branch.clone());
        vars.insert("worktree".to_string(), self.worktree.clone());
        vars.insert("repo_root".to_string(), self.repo_root.clone());
        vars.insert("session".to_string(), self.session.clone());
        vars.insert("window".to_string(), self.window.clone());
        vars.insert("status".to_string(), self.status.clone());
        vars.insert(
            "prev_status".to_string(),
            self.prev_status.clone().unwrap_or_default(),
        );
        vars.insert("timestamp".to_string(), self.timestamp.clone());
        vars.insert("backup_dir".to_string(), self.backup_dir.clone());
        vars
    }

    /// Expand variables in a script string
    ///
    /// Replaces all `${variable_name}` patterns with their values.
    /// Unknown variables are left unchanged.
    pub fn expand_variables(&self, script: &str) -> String {
        let vars = self.to_env_vars();
        let mut result = script.to_string();

        for (name, value) in &vars {
            let pattern = format!("${{{}}}", name);
            result = result.replace(&pattern, value);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_context_new() {
        let ctx = HookContext::new("my-task", "wt/my-task", "/path/to/wt", "/path/to/repo");

        assert_eq!(ctx.task, "my-task");
        assert_eq!(ctx.branch, "wt/my-task");
        assert_eq!(ctx.worktree, "/path/to/wt");
        assert_eq!(ctx.repo_root, "/path/to/repo");
    }

    #[test]
    fn test_hook_context_builder_pattern() {
        let ctx = HookContext::new("task", "branch", "/wt", "/repo")
            .with_session("my-session")
            .with_window("my-window")
            .with_status("running")
            .with_prev_status("pending")
            .with_timestamp("2024-01-01T00:00:00Z")
            .with_backup_dir("/backup");

        assert_eq!(ctx.session, "my-session");
        assert_eq!(ctx.window, "my-window");
        assert_eq!(ctx.status, "running");
        assert_eq!(ctx.prev_status, Some("pending".to_string()));
        assert_eq!(ctx.timestamp, "2024-01-01T00:00:00Z");
        assert_eq!(ctx.backup_dir, "/backup");
    }

    #[test]
    fn test_expand_variables_simple() {
        let ctx = HookContext::new("my-task", "wt/my-task", "/path/to/wt", "/path/to/repo");

        let script = "echo ${task}";
        let result = ctx.expand_variables(script);
        assert_eq!(result, "echo my-task");
    }

    #[test]
    fn test_expand_variables_multiple() {
        let ctx = HookContext::new("auth", "wt/auth", "/wt/auth", "/repo")
            .with_session("dev")
            .with_window("auth");

        let script = "Task: ${task}, Branch: ${branch}, Session: ${session}";
        let result = ctx.expand_variables(script);
        assert_eq!(result, "Task: auth, Branch: wt/auth, Session: dev");
    }

    #[test]
    fn test_expand_variables_multiline() {
        let ctx = HookContext::new("build", "wt/build", "/wt/build", "/repo");

        let script = r#"
cd ${worktree}
echo "Building ${task}..."
git push origin ${branch}
"#;
        let result = ctx.expand_variables(script);
        assert!(result.contains("cd /wt/build"));
        assert!(result.contains("Building build..."));
        assert!(result.contains("git push origin wt/build"));
    }

    #[test]
    fn test_expand_variables_unknown_preserved() {
        let ctx = HookContext::new("task", "branch", "/wt", "/repo");

        let script = "echo ${task} ${unknown_var}";
        let result = ctx.expand_variables(script);
        assert_eq!(result, "echo task ${unknown_var}");
    }

    #[test]
    fn test_expand_variables_all_vars() {
        let ctx = HookContext::new("task", "branch", "/wt", "/repo")
            .with_session("session")
            .with_window("window")
            .with_status("running")
            .with_prev_status("pending")
            .with_timestamp("timestamp")
            .with_backup_dir("/backup");

        let script = "${task} ${branch} ${worktree} ${repo_root} ${session} ${window} ${status} ${prev_status} ${timestamp} ${backup_dir}";
        let result = ctx.expand_variables(script);
        assert_eq!(
            result,
            "task branch /wt /repo session window running pending timestamp /backup"
        );
    }

    #[test]
    fn test_expand_variables_empty_prev_status() {
        let ctx = HookContext::new("task", "branch", "/wt", "/repo");

        let script = "prev: ${prev_status}";
        let result = ctx.expand_variables(script);
        assert_eq!(result, "prev: ");
    }

    #[test]
    fn test_to_env_vars() {
        let ctx = HookContext::new("task", "branch", "/wt", "/repo")
            .with_session("session")
            .with_status("running");

        let vars = ctx.to_env_vars();
        assert_eq!(vars.get("task"), Some(&"task".to_string()));
        assert_eq!(vars.get("branch"), Some(&"branch".to_string()));
        assert_eq!(vars.get("worktree"), Some(&"/wt".to_string()));
        assert_eq!(vars.get("repo_root"), Some(&"/repo".to_string()));
        assert_eq!(vars.get("session"), Some(&"session".to_string()));
        assert_eq!(vars.get("status"), Some(&"running".to_string()));
    }

    #[test]
    fn test_expand_variables_no_change_without_vars() {
        let ctx = HookContext::default();

        let script = "echo hello world";
        let result = ctx.expand_variables(script);
        assert_eq!(result, "echo hello world");
    }

    #[test]
    fn test_expand_variables_special_chars_in_value() {
        let ctx = HookContext::new("task-name", "wt/feature/task-name", "/path/with spaces", "/repo");

        let script = "cd \"${worktree}\" && git checkout ${branch}";
        let result = ctx.expand_variables(script);
        assert_eq!(
            result,
            "cd \"/path/with spaces\" && git checkout wt/feature/task-name"
        );
    }
}
