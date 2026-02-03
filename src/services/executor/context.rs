//! Execution context for step/workflow/phase execution.
//!
//! Provides variable expansion and environment setup for step/workflow/phase execution.

use std::collections::HashMap;

use crate::models::phase::ExitReason;

/// Execution context for step execution.
///
/// Contains all variables available for expansion in scripts and prompts.
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    // ========== Task Context ==========
    /// Task name
    pub task: String,
    /// Branch name
    pub branch: String,
    /// Worktree path
    pub worktree: String,
    /// Repository root path
    pub repo_root: String,

    // ========== Multiplexer Context ==========
    /// Multiplexer session name
    pub session: String,
    /// Multiplexer window name
    pub window: String,

    // ========== Phase Context ==========
    /// Current phase ID (e.g., "developing", "reviewing")
    pub phase: String,
    /// Exit reason (for on_exit workflows)
    pub exit_reason: Option<ExitReason>,

    // ========== Step Context ==========
    /// Current step index (0-based)
    pub step_index: usize,
    /// Current step ID (if named)
    pub step_id: Option<String>,
    /// Previous step's state
    pub prev_state: Option<String>,
    /// Previous step's stdout (for piping)
    pub prev_stdout: Option<String>,

    // ========== Workflow Context ==========
    /// Results from previous steps (step_id -> output)
    pub step_outputs: HashMap<String, String>,

    // ========== Custom Variables ==========
    /// Additional custom variables
    pub extra: HashMap<String, String>,
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
            exit_reason: None,
            step_index: 0,
            step_id: None,
            prev_state: None,
            prev_stdout: None,
            step_outputs: HashMap::new(),
            extra: HashMap::new(),
        }
    }
}

impl ExecutionContext {
    /// Create a new execution context with required fields.
    pub fn new(task: &str, branch: &str, worktree: &str, repo_root: &str) -> Self {
        Self {
            task: task.to_string(),
            branch: branch.to_string(),
            worktree: worktree.to_string(),
            repo_root: repo_root.to_string(),
            ..Default::default()
        }
    }

    // ========== Builder Methods ==========

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

    /// Set exit reason.
    pub fn with_exit_reason(mut self, reason: ExitReason) -> Self {
        self.exit_reason = Some(reason);
        self
    }

    // ========== Variable Expansion ==========

    /// Expand variables in a string.
    ///
    /// Supports:
    /// - `${task}`, `${branch}`, etc. - basic variables
    /// - `${prev.state}`, `${prev.stdout}` - previous step info
    /// - `${steps.step_id.output}` - step outputs
    /// - `${phase.exit_reason}` - phase info
    ///
    /// Unknown variables are left unchanged.
    pub fn expand(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Basic variables
        result = result.replace("${task}", &self.task);
        result = result.replace("${branch}", &self.branch);
        result = result.replace("${worktree}", &self.worktree);
        result = result.replace("${repo_root}", &self.repo_root);
        result = result.replace("${session}", &self.session);
        result = result.replace("${window}", &self.window);
        result = result.replace("${phase}", &self.phase);
        result = result.replace("${step_index}", &self.step_index.to_string());

        // Step ID
        if let Some(ref id) = self.step_id {
            result = result.replace("${step_id}", id);
        }

        // Previous step info
        if let Some(ref state) = self.prev_state {
            result = result.replace("${prev.state}", state);
        }
        if let Some(ref stdout) = self.prev_stdout {
            result = result.replace("${prev.stdout}", stdout);
        }

        // Exit reason
        if let Some(ref reason) = self.exit_reason {
            let reason_str = match reason {
                ExitReason::Success => "success",
                ExitReason::Forced => "forced",
                ExitReason::Failed => "failed",
            };
            result = result.replace("${phase.exit_reason}", reason_str);
            result = result.replace("${exit_reason}", reason_str);
        }

        // Step outputs: ${steps.step_id.output}
        for (step_id, output) in &self.step_outputs {
            result = result.replace(&format!("${{steps.{}.output}}", step_id), output);
        }

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

        // Basic variables
        vars.insert("WT_TASK".to_string(), self.task.clone());
        vars.insert("WT_BRANCH".to_string(), self.branch.clone());
        vars.insert("WT_WORKTREE".to_string(), self.worktree.clone());
        vars.insert("WT_REPO_ROOT".to_string(), self.repo_root.clone());
        vars.insert("WT_SESSION".to_string(), self.session.clone());
        vars.insert("WT_WINDOW".to_string(), self.window.clone());
        vars.insert("WT_PHASE".to_string(), self.phase.clone());
        vars.insert("WT_STEP_INDEX".to_string(), self.step_index.to_string());

        // Optional variables
        if let Some(ref id) = self.step_id {
            vars.insert("WT_STEP_ID".to_string(), id.clone());
        }
        if let Some(ref state) = self.prev_state {
            vars.insert("WT_PREV_STATE".to_string(), state.clone());
        }
        if let Some(ref reason) = self.exit_reason {
            let reason_str = match reason {
                ExitReason::Success => "success",
                ExitReason::Forced => "forced",
                ExitReason::Failed => "failed",
            };
            vars.insert("WT_EXIT_REASON".to_string(), reason_str.to_string());
        }

        // Custom variables
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

    /// Clone context for next step execution.
    pub fn next_step(&self, index: usize, step_id: Option<&str>) -> Self {
        Self {
            task: self.task.clone(),
            branch: self.branch.clone(),
            worktree: self.worktree.clone(),
            repo_root: self.repo_root.clone(),
            session: self.session.clone(),
            window: self.window.clone(),
            phase: self.phase.clone(),
            exit_reason: self.exit_reason.clone(),
            step_index: index,
            step_id: step_id.map(String::from),
            prev_state: None,
            prev_stdout: None,
            step_outputs: self.step_outputs.clone(),
            extra: self.extra.clone(),
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
        assert_eq!(ctx.step_index, 0);
    }

    #[test]
    fn test_context_builder() {
        let ctx = ExecutionContext::new("auth", "wt/auth", "/work/auth", "/repo")
            .with_session("wt")
            .with_phase("developing");

        assert_eq!(ctx.session, "wt");
        assert_eq!(ctx.phase, "developing");
    }

    #[test]
    fn test_expand_basic_variables() {
        let mut ctx = ExecutionContext::new("auth", "wt/auth", "/work/auth", "/repo")
            .with_session("wt")
            .with_phase("developing");
        ctx.step_index = 1;

        assert_eq!(ctx.expand("task: ${task}"), "task: auth");
        assert_eq!(ctx.expand("@.wt/tasks/${task}.md"), "@.wt/tasks/auth.md");
        assert_eq!(ctx.expand("step ${step_index}"), "step 1");
        assert_eq!(ctx.expand("phase: ${phase}"), "phase: developing");
    }

    #[test]
    fn test_expand_exit_reason() {
        let ctx = ExecutionContext::new("auth", "wt/auth", "/work/auth", "/repo")
            .with_exit_reason(ExitReason::Forced);

        assert_eq!(ctx.expand("${exit_reason}"), "forced");
        assert_eq!(ctx.expand("${phase.exit_reason}"), "forced");
    }

    #[test]
    fn test_expand_unknown_preserved() {
        let ctx = ExecutionContext::new("auth", "wt/auth", "/work/auth", "/repo");
        assert_eq!(ctx.expand("${unknown}"), "${unknown}");
    }

    #[test]
    fn test_to_env_vars() {
        let mut ctx = ExecutionContext::new("auth", "wt/auth", "/work/auth", "/repo")
            .with_session("wt")
            .with_exit_reason(ExitReason::Success);
        ctx.step_index = 3;
        ctx.step_id = Some("build".to_string());

        let vars = ctx.to_env_vars();
        assert_eq!(vars.get("WT_TASK"), Some(&"auth".to_string()));
        assert_eq!(vars.get("WT_BRANCH"), Some(&"wt/auth".to_string()));
        assert_eq!(vars.get("WT_SESSION"), Some(&"wt".to_string()));
        assert_eq!(vars.get("WT_STEP_INDEX"), Some(&"3".to_string()));
        assert_eq!(vars.get("WT_STEP_ID"), Some(&"build".to_string()));
        assert_eq!(vars.get("WT_EXIT_REASON"), Some(&"success".to_string()));
    }

    #[test]
    fn test_working_dir() {
        let ctx = ExecutionContext::default();
        assert_eq!(ctx.working_dir(), ".");

        let ctx = ExecutionContext::new("", "", "", "/repo");
        assert_eq!(ctx.working_dir(), "/repo");
    }

    #[test]
    fn test_next_step() {
        let ctx = ExecutionContext::new("auth", "wt/auth", "/work/auth", "/repo")
            .with_phase("developing");

        let next = ctx.next_step(1, Some("step2"));
        assert_eq!(next.task, "auth");
        assert_eq!(next.phase, "developing");
        assert_eq!(next.step_index, 1);
        assert_eq!(next.step_id, Some("step2".to_string()));
        assert!(next.prev_state.is_none());
    }
}
