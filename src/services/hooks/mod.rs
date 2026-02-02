//! Hooks execution engine.
//!
//! Executes hooks defined in `.wt/config.jsonc` with support for:
//! - Multiple step types (script, agent, internal, condition)
//! - Pipeline mode (stream-json chaining)
//! - Automatic state management

pub mod context;
pub mod pipeline;
pub mod step;

pub use context::ExecutionContext;
pub use step::StepExecutor;

use crate::error::{Result, WtError};
use crate::models::{WtConfig, HookDef, IdleReason, Step, TaskPhase, TaskState};

/// Default state transitions for each command
#[derive(Debug, Clone)]
pub struct DefaultTransition {
    /// Phase to set at start
    pub start_phase: Option<TaskPhase>,
    /// Phase on success (None = keep current)
    pub success_phase: Option<TaskPhase>,
    /// IdleReason on success
    pub success_reason: IdleReason,
    /// IdleReason on failure
    pub failure_reason: IdleReason,
}

impl DefaultTransition {
    /// Get default transition for a command
    pub fn for_command(command: &str) -> Option<Self> {
        match command {
            "run" => Some(Self {
                start_phase: Some(TaskPhase::Developing),
                success_phase: None, // Keep developing
                success_reason: IdleReason::Done,
                failure_reason: IdleReason::Error,
            }),
            "review" => Some(Self {
                start_phase: Some(TaskPhase::Reviewing),
                success_phase: None,
                success_reason: IdleReason::Done,
                failure_reason: IdleReason::Error,
            }),
            "resume" => Some(Self {
                start_phase: None, // Keep current phase
                success_phase: None,
                success_reason: IdleReason::Done,
                failure_reason: IdleReason::Error,
            }),
            "complete" => Some(Self {
                start_phase: Some(TaskPhase::Merging),
                success_phase: Some(TaskPhase::None), // Will become Completed
                success_reason: IdleReason::Done,
                failure_reason: IdleReason::Conflict,
            }),
            "delete" | "reset" => None, // Special handling
            _ => None,
        }
    }
}

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

        match hook_def {
            HookDef::Steps(steps) => self.execute_steps(steps, context),
            HookDef::Pipeline { pipeline } => self.execute_pipeline(pipeline, context),
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

    /// Execute a hook with automatic state management
    pub fn execute_with_state(
        &self,
        hook_name: &str,
        context: &ExecutionContext,
        state: &mut TaskState,
    ) -> Result<()> {
        let transition = DefaultTransition::for_command(hook_name);

        // Set start state
        if let Some(t) = &transition {
            if let Some(phase) = &t.start_phase {
                state.to_active(phase.clone());
            } else {
                // Keep phase, just go active
                let current_phase = state.phase.clone();
                state.to_active(current_phase);
            }
        }

        // Execute the hook
        let result = self.execute(hook_name, context);

        // Set end state based on result
        match (&result, &transition) {
            (Ok(()), Some(t)) => {
                // Success
                if let Some(phase) = &t.success_phase {
                    state.phase = phase.clone();
                }
                state.to_idle(t.success_reason.clone());
            }
            (Err(_), Some(t)) => {
                // Failure
                state.to_idle(t.failure_reason.clone());
            }
            _ => {
                // No transition defined or special command
            }
        }

        result
    }

    /// Check if a hook is defined
    pub fn has_hook(&self, hook_name: &str) -> bool {
        self.config.get_hook(hook_name).is_some()
    }

    // =========================================================================
    // Convenience methods
    // =========================================================================

    /// Execute the 'run' hook
    pub fn run(&self, context: &ExecutionContext) -> Result<()> {
        self.execute("run", context)
    }

    /// Execute the 'review' hook
    pub fn review(&self, context: &ExecutionContext) -> Result<()> {
        self.execute("review", context)
    }

    /// Execute the 'resume' hook
    pub fn resume(&self, context: &ExecutionContext) -> Result<()> {
        self.execute("resume", context)
    }

    /// Execute the 'complete' hook
    pub fn complete(&self, context: &ExecutionContext) -> Result<()> {
        self.execute("complete", context)
    }

    /// Execute the 'delete' hook
    pub fn delete(&self, context: &ExecutionContext) -> Result<()> {
        self.execute("delete", context)
    }

    /// Execute the 'reset' hook
    pub fn reset(&self, context: &ExecutionContext) -> Result<()> {
        self.execute("reset", context)
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
        assert!(engine.run(&context).is_ok());
        assert!(engine.review(&context).is_ok());
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

        assert!(engine.run(&context).is_ok());
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

        let result = engine.run(&context);
        assert!(result.is_err());
    }

    #[test]
    fn test_engine_with_state_success() {
        let config = test_config_with_hooks(
            r#"{
            "run": [{"type": "script", "run": "true"}]
        }"#,
        );
        let engine = HooksEngine::new(&config);
        let context = test_context();
        let mut state = TaskState::pending();

        let result = engine.execute_with_state("run", &context, &mut state);
        assert!(result.is_ok());

        // State should be Idle after success
        assert_eq!(state.status, crate::models::TaskStatus::Idle);
        assert_eq!(state.phase, TaskPhase::Developing);
        assert_eq!(state.idle_reason, Some(IdleReason::Done));
    }

    #[test]
    fn test_engine_with_state_failure() {
        let config = test_config_with_hooks(
            r#"{
            "run": [{"type": "script", "run": "exit 1"}]
        }"#,
        );
        let engine = HooksEngine::new(&config);
        let context = test_context();
        let mut state = TaskState::pending();

        let result = engine.execute_with_state("run", &context, &mut state);
        assert!(result.is_err());

        // State should be Idle with error
        assert_eq!(state.status, crate::models::TaskStatus::Idle);
        assert_eq!(state.idle_reason, Some(IdleReason::Error));
    }

    #[test]
    fn test_default_transition_run() {
        let t = DefaultTransition::for_command("run").unwrap();
        assert_eq!(t.start_phase, Some(TaskPhase::Developing));
        assert_eq!(t.success_reason, IdleReason::Done);
        assert_eq!(t.failure_reason, IdleReason::Error);
    }

    #[test]
    fn test_default_transition_complete() {
        let t = DefaultTransition::for_command("complete").unwrap();
        assert_eq!(t.start_phase, Some(TaskPhase::Merging));
        assert_eq!(t.failure_reason, IdleReason::Conflict);
    }

    #[test]
    fn test_default_transition_unknown() {
        assert!(DefaultTransition::for_command("unknown").is_none());
        assert!(DefaultTransition::for_command("delete").is_none());
    }
}
