//! Hooks execution engine.
//!
//! Executes hooks defined in `.wt/config.jsonc` with support for:
//! - Multiple step types (script, agent, internal, condition)
//! - Pipeline mode (stream-json chaining)
//! - Automatic state management

pub mod context;
pub mod pipeline;
mod pipeline_store;
pub mod step;

pub use context::ExecutionContext;
pub use pipeline::{cleanup_pipelines, kill_pipeline, list_pipelines, PipelineState};
pub use step::StepExecutor;

use crate::error::{Result, WtError};
use crate::models::{WtConfig, HookDef, Step};

/// Hooks execution engine
pub struct HooksEngine<'a> {
    config: &'a WtConfig,
}

impl<'a> HooksEngine<'a> {
    /// Create a new hooks engine
    pub fn new(config: &'a WtConfig) -> Self {
        Self { config }
    }

    /// Execute a hook by name
    pub fn execute(&self, hook_name: &str, context: &ExecutionContext) -> Result<()> {
        let hook_def = match self.config.get_hook(hook_name) {
            Some(h) => h,
            None => return Ok(()), // No hook defined, use defaults
        };

        // Resolve pipeline references
        let resolved = self.config.resolve_hook(hook_def).ok_or_else(|| {
            WtError::HookFailed {
                hook: hook_name.to_string(),
                message: "Failed to resolve pipeline reference".to_string(),
                exit_code: None,
            }
        })?;

        match &resolved {
            HookDef::Steps(steps) => self.execute_steps(steps, context),
            HookDef::Pipeline { pipeline } => self.execute_pipeline(pipeline, context),
            HookDef::PipelineRef { use_pipeline } => {
                // This shouldn't happen after resolve, but handle it
                Err(WtError::HookFailed {
                    hook: hook_name.to_string(),
                    message: format!("Unknown pipeline: {}", use_pipeline),
                    exit_code: None,
                })
            }
        }
    }

    /// Execute a sequence of steps
    fn execute_steps(&self, steps: &[Step], context: &ExecutionContext) -> Result<()> {
        let executor = StepExecutor::new(self.config, context);

        for (i, step) in steps.iter().enumerate() {
            match executor.execute(step) {
                Ok(_) => {}
                Err(e) => {
                    return Err(WtError::HookFailed {
                        hook: format!("step {}", i + 1),
                        message: e.to_string(),
                        exit_code: None,
                    });
                }
            }
        }

        Ok(())
    }

    /// Execute a pipeline of agents
    fn execute_pipeline(&self, steps: &[Step], context: &ExecutionContext) -> Result<()> {
        let executor = pipeline::PipelineExecutor::new(self.config, context);
        executor.execute(steps)
    }

    /// Check if a hook is defined
    pub fn has_hook(&self, hook_name: &str) -> bool {
        self.config.get_hook(hook_name).is_some()
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config_with_hooks(hooks_json: &str) -> WtConfig {
        let json = format!(
            r#"{{
            "multiplexer": "tmux",
            "session_name": "test",
            "hooks": {}
        }}"#,
            hooks_json
        );
        WtConfig::from_str(&json).unwrap()
    }

    fn test_context() -> ExecutionContext {
        // Use current directory for tests
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string());
        ExecutionContext::new("auth", "wt/auth", &cwd, &cwd)
            .with_session("wt")
            .with_window("auth")
    }

    #[test]
    fn test_engine_no_hooks() {
        let config = WtConfig::default();
        let engine = HooksEngine::new(&config);
        let context = test_context();

        // Should succeed when no hook defined
        assert!(engine.execute("run", &context).is_ok());
        assert!(engine.execute("review", &context).is_ok());
    }

    #[test]
    fn test_engine_has_hook() {
        let config = test_config_with_hooks(
            r#"{
            "run": [{"type": "script", "run": "true"}]
        }"#,
        );
        let engine = HooksEngine::new(&config);

        assert!(engine.has_hook("run"));
        assert!(!engine.has_hook("review"));
    }

    #[test]
    fn test_engine_execute_steps() {
        let config = test_config_with_hooks(
            r#"{
            "run": [
                {"type": "script", "run": "true"},
                {"type": "script", "run": "true"}
            ]
        }"#,
        );
        let engine = HooksEngine::new(&config);
        let context = test_context();

        assert!(engine.execute("run", &context).is_ok());
    }

    #[test]
    fn test_engine_execute_steps_failure() {
        let config = test_config_with_hooks(
            r#"{
            "run": [
                {"type": "script", "run": "true"},
                {"type": "script", "run": "exit 1"}
            ]
        }"#,
        );
        let engine = HooksEngine::new(&config);
        let context = test_context();

        let result = engine.execute("run", &context);
        assert!(result.is_err());
    }
}
