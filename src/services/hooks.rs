//! Hooks execution engine.
//!
//! Executes lifecycle hooks with variable expansion and proper error handling.

use std::process::{Command, Stdio};

use crate::error::{Result, WtError};
use crate::models::{HookContext, HookName, WtConfig};

/// Hooks execution engine.
///
/// Manages the execution of lifecycle hooks defined in the configuration.
pub struct HooksEngine<'a> {
    config: &'a WtConfig,
}

impl<'a> HooksEngine<'a> {
    /// Create a new hooks engine with the given configuration.
    pub fn new(config: &'a WtConfig) -> Self {
        Self { config }
    }

    /// Execute a hook by name.
    ///
    /// Returns Ok(()) if:
    /// - The hook is not defined (silently skipped)
    /// - The hook script executes successfully (exit code 0)
    ///
    /// Returns Err if:
    /// - The hook script fails (non-zero exit code)
    /// - The script cannot be executed
    pub fn run_hook(&self, hook: HookName, context: &HookContext) -> Result<()> {
        let script = match self.config.get_hook(hook) {
            Some(s) => s,
            None => return Ok(()), // No hook defined, skip
        };

        let expanded = context.expand_variables(script);
        self.execute_script(hook, &expanded, context)
    }

    /// Execute a script string.
    fn execute_script(&self, hook: HookName, script: &str, context: &HookContext) -> Result<()> {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(script);

        // Set working directory to repo_root if available and exists
        if !context.repo_root.is_empty() {
            let repo_path = std::path::Path::new(&context.repo_root);
            if repo_path.exists() {
                cmd.current_dir(&context.repo_root);
            }
        }

        // Set environment variables for the script
        for (key, value) in context.to_env_vars() {
            cmd.env(format!("WT_{}", key.to_uppercase()), value);
        }

        // Stream output to terminal
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let status = cmd
            .spawn()
            .and_then(|mut child| child.wait())
            .map_err(|e| WtError::HookFailed {
                hook: hook.as_str().to_string(),
                message: format!("Failed to execute: {}", e),
                exit_code: None,
            })?;

        if status.success() {
            Ok(())
        } else {
            Err(WtError::HookFailed {
                hook: hook.as_str().to_string(),
                message: "Script exited with non-zero status".to_string(),
                exit_code: status.code(),
            })
        }
    }

    // =========================================================================
    // Convenience methods for each hook
    // =========================================================================

    /// Run the on_create hook (after worktree is created).
    pub fn on_create(&self, context: &HookContext) -> Result<()> {
        self.run_hook(HookName::OnCreate, context)
    }

    /// Run the before_run hook (before starting the task).
    pub fn before_run(&self, context: &HookContext) -> Result<()> {
        self.run_hook(HookName::BeforeRun, context)
    }

    /// Run the after_run hook (after task starts running).
    pub fn after_run(&self, context: &HookContext) -> Result<()> {
        self.run_hook(HookName::AfterRun, context)
    }

    /// Run the before_review hook (before marking task for review).
    pub fn before_review(&self, context: &HookContext) -> Result<()> {
        self.run_hook(HookName::BeforeReview, context)
    }

    /// Run the after_review hook (after task is marked for review).
    pub fn after_review(&self, context: &HookContext) -> Result<()> {
        self.run_hook(HookName::AfterReview, context)
    }

    /// Run the before_resume hook (before resuming from review).
    pub fn before_resume(&self, context: &HookContext) -> Result<()> {
        self.run_hook(HookName::BeforeResume, context)
    }

    /// Run the before_complete hook (before completing/merging).
    pub fn before_complete(&self, context: &HookContext) -> Result<()> {
        self.run_hook(HookName::BeforeComplete, context)
    }

    /// Run the after_complete hook (after task is completed).
    pub fn after_complete(&self, context: &HookContext) -> Result<()> {
        self.run_hook(HookName::AfterComplete, context)
    }

    /// Run the before_delete hook (before deleting worktree).
    pub fn before_delete(&self, context: &HookContext) -> Result<()> {
        self.run_hook(HookName::BeforeDelete, context)
    }

    /// Run the before_reset hook (before resetting task).
    pub fn before_reset(&self, context: &HookContext) -> Result<()> {
        self.run_hook(HookName::BeforeReset, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(hooks_yaml: &str) -> WtConfig {
        let yaml = format!(
            r#"
session_name: test
{}"#,
            hooks_yaml
        );
        WtConfig::from_str(&yaml).unwrap()
    }

    fn test_context() -> HookContext {
        HookContext::new("test-task", "wt/test-task", "/tmp/worktree", "/tmp/repo")
            .with_session("test-session")
            .with_window("test-window")
            .with_status("running")
    }

    #[test]
    fn test_run_hook_no_hook_defined() {
        let config = test_config("");
        let engine = HooksEngine::new(&config);
        let context = test_context();

        // Should succeed silently when no hook is defined
        assert!(engine.on_create(&context).is_ok());
        assert!(engine.before_review(&context).is_ok());
    }

    #[test]
    fn test_run_hook_simple_success() {
        let config = test_config(
            r#"
hooks:
  on_create: "true"
"#,
        );
        let engine = HooksEngine::new(&config);
        let context = test_context();

        assert!(engine.on_create(&context).is_ok());
    }

    #[test]
    fn test_run_hook_failure() {
        let config = test_config(
            r#"
hooks:
  on_create: "exit 1"
"#,
        );
        let engine = HooksEngine::new(&config);
        let context = test_context();

        let result = engine.on_create(&context);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, WtError::HookFailed { .. }));
    }

    #[test]
    fn test_run_hook_with_variables() {
        let config = test_config(
            r#"
hooks:
  on_create: "echo ${task}"
"#,
        );
        let engine = HooksEngine::new(&config);
        let context = test_context();

        // Should succeed - echo always returns 0
        assert!(engine.on_create(&context).is_ok());
    }

    #[test]
    fn test_run_hook_multiline_script() {
        let config = test_config(
            r#"
hooks:
  before_review: |
    echo "step 1"
    echo "step 2"
    true
"#,
        );
        let engine = HooksEngine::new(&config);
        let context = test_context();

        assert!(engine.before_review(&context).is_ok());
    }

    #[test]
    fn test_run_hook_multiline_script_fails_on_error() {
        let config = test_config(
            r#"
hooks:
  before_review: |
    echo "step 1"
    exit 42
    echo "step 2"
"#,
        );
        let engine = HooksEngine::new(&config);
        let context = test_context();

        let result = engine.before_review(&context);
        assert!(result.is_err());

        if let Err(WtError::HookFailed { exit_code, .. }) = result {
            assert_eq!(exit_code, Some(42));
        } else {
            panic!("Expected HookFailed error");
        }
    }

    #[test]
    fn test_legacy_fallback() {
        // Test that legacy fields work through get_hook
        let config = test_config(
            r#"
init_script: "echo legacy init"
review_script: "echo legacy review"
"#,
        );
        let engine = HooksEngine::new(&config);
        let context = test_context();

        // on_create should use init_script fallback
        assert!(engine.on_create(&context).is_ok());
        // before_review should use review_script fallback
        assert!(engine.before_review(&context).is_ok());
    }
}
