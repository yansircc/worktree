//! Step executor for Phases v2.
//!
//! Executes individual steps with support for:
//! - Script execution
//! - Agent execution
//! - Verification (self, script, agent, human)
//! - Observation (terminal, log)

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Instant;

use crate::models::step::{Step, StepResult, StepState, StepVerify};
use crate::models::WtConfig;
use crate::services::claude::ClaudeCommandBuilder;
use crate::services::executor::condition::ConditionEvaluator;
use crate::services::executor::context::ExecutionContext;

/// Step executor
pub struct StepExecutor<'a> {
    config: &'a WtConfig,
    context: ExecutionContext,
    log_dir: Option<PathBuf>,
}

impl<'a> StepExecutor<'a> {
    /// Create a new step executor.
    pub fn new(config: &'a WtConfig, context: ExecutionContext) -> Self {
        Self {
            config,
            context,
            log_dir: None,
        }
    }

    /// Set log directory for step output.
    pub fn with_log_dir(mut self, dir: PathBuf) -> Self {
        self.log_dir = Some(dir);
        self
    }

    /// Execute a step and return the result.
    pub fn execute(&self, step: &Step) -> StepResult {
        let start = Instant::now();
        let step_id = step.id.clone();

        // Check condition using enhanced evaluator
        if let Some(ref condition) = step.condition {
            let context = self.context.clone();
            let working_dir = self.context.working_dir().to_string();
            let evaluator = ConditionEvaluator::new(
                move |s| context.expand(s),
                &working_dir,
            );

            if !evaluator.evaluate(condition) {
                return StepResult {
                    step_id,
                    state: StepState::Skipped,
                    message: Some(format!("Condition not met: {}", condition)),
                    duration_ms: start.elapsed().as_millis() as u64,
                    ..Default::default()
                };
            }
        }

        // Determine output file
        let output_file = self.get_output_file(&step_id);

        // Execute based on type
        let (state, exit_code, message) = if let Some(ref script) = step.run {
            self.execute_script(script, &output_file)
        } else if let Some(ref agent) = step.agent {
            self.execute_agent(agent, &output_file)
        } else {
            (
                StepState::Failed,
                None,
                Some("Step has no executor (run or agent)".to_string()),
            )
        };

        // If execution succeeded, run verification
        let (final_state, final_message) = if state == StepState::Success {
            if let Some(ref verify) = step.verify {
                self.run_verification(verify, &output_file)
            } else {
                (state, message)
            }
        } else {
            (state, message)
        };

        StepResult {
            step_id,
            state: final_state,
            exit_code,
            message: final_message,
            output_file,
            artifacts: Vec::new(), // TODO: collect artifacts
            exports: std::collections::HashMap::new(), // TODO: extract exports
            duration_ms: start.elapsed().as_millis() as u64,
            attempt: 0,
        }
    }

    /// Get output file path for this step.
    fn get_output_file(&self, step_id: &Option<String>) -> PathBuf {
        if let Some(ref dir) = self.log_dir {
            let filename = if let Some(ref id) = step_id {
                format!("step-{}-{}.log", self.context.step_index, id)
            } else {
                format!("step-{}.log", self.context.step_index)
            };
            dir.join(filename)
        } else {
            PathBuf::new()
        }
    }

    /// Write command output to log file.
    fn write_output_log(output_file: &PathBuf, output: &std::process::Output) {
        if output_file.as_os_str().is_empty() {
            return;
        }
        if let Some(parent) = output_file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = std::fs::File::create(output_file) {
            let _ = file.write_all(&output.stdout);
            let _ = file.write_all(&output.stderr);
        }
    }

    /// Process command output into step result.
    fn process_output(
        output: &std::process::Output,
        error_prefix: &str,
    ) -> (StepState, Option<i32>, Option<String>) {
        let exit_code = output.status.code();
        if output.status.success() {
            (StepState::Success, exit_code, None)
        } else {
            (
                StepState::Failed,
                exit_code,
                Some(format!("{}: {}", error_prefix, exit_code.unwrap_or(-1))),
            )
        }
    }

    /// Configure command with context environment and output handling.
    fn configure_command(&self, cmd: &mut Command, output_file: &PathBuf, capture: bool) {
        for (key, value) in self.context.to_env_vars() {
            cmd.env(key, value);
        }
        if output_file.as_os_str().is_empty() || !capture {
            cmd.stdout(Stdio::inherit());
            cmd.stderr(Stdio::inherit());
        } else {
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());
        }
    }

    /// Execute a shell script.
    fn execute_script(
        &self,
        script: &str,
        output_file: &PathBuf,
    ) -> (StepState, Option<i32>, Option<String>) {
        let expanded = self.context.expand(script);

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(&expanded);
        cmd.current_dir(self.context.working_dir());
        self.configure_command(&mut cmd, output_file, true);

        match cmd.spawn().and_then(|child| child.wait_with_output()) {
            Ok(output) => {
                Self::write_output_log(output_file, &output);
                Self::process_output(&output, "Exit code")
            }
            Err(e) => (
                StepState::Failed,
                None,
                Some(format!("Failed to execute: {}", e)),
            ),
        }
    }

    /// Execute a Claude agent.
    fn execute_agent(
        &self,
        agent: &crate::models::AgentStep,
        output_file: &PathBuf,
    ) -> (StepState, Option<i32>, Option<String>) {
        let expanded_prompt = self.context.expand(&agent.prompt);

        // Build command
        let builder = ClaudeCommandBuilder::from_agent_step(agent, &self.context);
        let args = builder.prompt(&expanded_prompt).build();

        let mut cmd = Command::new(&self.config.claude_command);
        cmd.args(&args);
        cmd.current_dir(self.context.working_dir());
        self.configure_command(&mut cmd, output_file, agent.print);

        match cmd.spawn().and_then(|child| child.wait_with_output()) {
            Ok(output) => {
                if agent.print {
                    Self::write_output_log(output_file, &output);
                }
                Self::process_output(&output, "Agent exited with code")
            }
            Err(e) => (
                StepState::Failed,
                None,
                Some(format!("Failed to execute agent: {}", e)),
            ),
        }
    }

    /// Run verification on step output.
    fn run_verification(
        &self,
        verify: &StepVerify,
        _output_file: &PathBuf,
    ) -> (StepState, Option<String>) {
        match verify {
            StepVerify::SelfMark => {
                // Agent self-marks via wt step command - assume success for now
                (StepState::Success, None)
            }
            StepVerify::Script { run, on_pass, on_fail } => {
                let expanded = self.context.expand(run);
                let mut cmd = Command::new("sh");
                cmd.arg("-c").arg(&expanded);
                cmd.current_dir(self.context.working_dir());

                for (key, value) in self.context.to_env_vars() {
                    cmd.env(key, value);
                }

                match cmd.status() {
                    Ok(status) if status.success() => {
                        let state = match on_pass {
                            crate::models::step::VerifyAction::Success => StepState::Success,
                            crate::models::step::VerifyAction::Failed => StepState::Failed,
                            crate::models::step::VerifyAction::Blocked => StepState::Blocked,
                            crate::models::step::VerifyAction::Retry => StepState::Pending,
                        };
                        (state, None)
                    }
                    Ok(_) => {
                        let state = match on_fail {
                            crate::models::step::VerifyAction::Success => StepState::Success,
                            crate::models::step::VerifyAction::Failed => StepState::Failed,
                            crate::models::step::VerifyAction::Blocked => StepState::Blocked,
                            crate::models::step::VerifyAction::Retry => StepState::Pending,
                        };
                        (state, Some("Verification failed".to_string()))
                    }
                    Err(e) => (StepState::Failed, Some(format!("Verification error: {}", e))),
                }
            }
            StepVerify::Agent { .. } => {
                // TODO: Run agent for verification
                (StepState::Success, None)
            }
            StepVerify::Human { prompt, .. } => {
                // Human verification - mark as blocked
                (StepState::Blocked, Some(format!("Awaiting human review: {}", prompt)))
            }
            StepVerify::Schema { .. } => {
                // TODO: Validate output against schema
                (StepState::Success, None)
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> WtConfig {
        WtConfig::default()
    }

    fn test_context() -> ExecutionContext {
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string());
        ExecutionContext::new("auth", "wt/auth", &cwd, &cwd)
            .with_session("wt")
            .with_window("auth")
            .with_phase("developing")
    }

    #[test]
    fn test_execute_script_success() {
        let config = test_config();
        let context = test_context();
        let executor = StepExecutor::new(&config, context);

        let step = Step::script("true");
        let result = executor.execute(&step);
        assert_eq!(result.state, StepState::Success);
    }

    #[test]
    fn test_execute_script_failure() {
        let config = test_config();
        let context = test_context();
        let executor = StepExecutor::new(&config, context);

        let step = Step::script("exit 1");
        let result = executor.execute(&step);
        assert_eq!(result.state, StepState::Failed);
        assert_eq!(result.exit_code, Some(1));
    }

    #[test]
    fn test_execute_script_with_variables() {
        let config = test_config();
        let context = test_context();
        let executor = StepExecutor::new(&config, context);

        let step = Step::script("test '${task}' = 'auth'");
        let result = executor.execute(&step);
        assert_eq!(result.state, StepState::Success);
    }

    #[test]
    fn test_execute_with_condition_true() {
        let config = test_config();
        let context = test_context();
        let executor = StepExecutor::new(&config, context);

        let mut step = Step::script("true");
        step.condition = Some("true".to_string());
        let result = executor.execute(&step);
        assert_eq!(result.state, StepState::Success);
    }

    #[test]
    fn test_execute_with_condition_false() {
        let config = test_config();
        let context = test_context();
        let executor = StepExecutor::new(&config, context);

        let mut step = Step::script("true");
        step.condition = Some("false".to_string());
        let result = executor.execute(&step);
        assert_eq!(result.state, StepState::Skipped);
    }

    #[test]
    fn test_condition_equality() {
        let mut context = test_context();
        context.prev_state = Some("success".to_string());
        let working_dir = context.working_dir().to_string();

        let evaluator = ConditionEvaluator::new(
            move |s| context.expand(s),
            &working_dir,
        );

        assert!(evaluator.evaluate("'success' == 'success'"));
        assert!(!evaluator.evaluate("'success' == 'failed'"));
        assert!(evaluator.evaluate("'success' != 'failed'"));
    }

    #[test]
    fn test_condition_with_variables() {
        let context = test_context();
        let working_dir = context.working_dir().to_string();

        let evaluator = ConditionEvaluator::new(
            move |s| context.expand(s),
            &working_dir,
        );

        // Test variable expansion in condition
        assert!(evaluator.evaluate("\"${task}\" == \"auth\""));
        assert!(evaluator.evaluate("\"${phase}\" == \"developing\""));
    }

    #[test]
    fn test_condition_logical_operators() {
        let context = test_context();
        let working_dir = context.working_dir().to_string();

        let evaluator = ConditionEvaluator::new(
            move |s| context.expand(s),
            &working_dir,
        );

        assert!(evaluator.evaluate("'a' == 'a' && 'b' == 'b'"));
        assert!(evaluator.evaluate("'a' == 'x' || 'b' == 'b'"));
        assert!(evaluator.evaluate("!'a' == 'b'"));
    }

    #[test]
    fn test_step_result_duration() {
        let config = test_config();
        let context = test_context();
        let executor = StepExecutor::new(&config, context);

        let step = Step::script("sleep 0.1");
        let result = executor.execute(&step);
        assert!(result.duration_ms >= 100);
    }
}
