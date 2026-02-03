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

use crate::error::Result;
use crate::models::step::{Step, StepResult, StepState, StepVerify};
use crate::models::WtConfig;
use crate::services::claude::ClaudeCommandBuilder;
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

        // Check condition
        if let Some(ref condition) = step.condition {
            if !self.evaluate_condition(condition) {
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
        }
    }

    /// Evaluate a condition expression.
    fn evaluate_condition(&self, condition: &str) -> bool {
        let expanded = self.context.expand(condition);

        // Simple expression evaluation
        // Supports: "value1 == value2", "value1 != value2", shell command
        if expanded.contains("==") {
            let parts: Vec<&str> = expanded.split("==").map(|s| s.trim()).collect();
            if parts.len() == 2 {
                return parts[0].trim_matches('\'').trim_matches('"')
                    == parts[1].trim_matches('\'').trim_matches('"');
            }
        }
        if expanded.contains("!=") {
            let parts: Vec<&str> = expanded.split("!=").map(|s| s.trim()).collect();
            if parts.len() == 2 {
                return parts[0].trim_matches('\'').trim_matches('"')
                    != parts[1].trim_matches('\'').trim_matches('"');
            }
        }

        // Fall back to shell command
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(&expanded);
        cmd.current_dir(self.context.working_dir());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());

        cmd.status().map(|s| s.success()).unwrap_or(false)
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

    /// Execute a shell script.
    fn execute_script(
        &self,
        script: &str,
        output_file: &PathBuf,
    ) -> (StepState, Option<i32>, Option<String>) {
        let expanded = self.context.expand(script);
        let working_dir = self.context.working_dir();

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(&expanded);
        cmd.current_dir(working_dir);

        // Set environment variables
        for (key, value) in self.context.to_env_vars() {
            cmd.env(key, value);
        }

        // Configure output
        if output_file.as_os_str().is_empty() {
            // Stream to terminal
            cmd.stdout(Stdio::inherit());
            cmd.stderr(Stdio::inherit());
        } else {
            // Capture output
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());
        }

        match cmd.spawn().and_then(|child| child.wait_with_output()) {
            Ok(output) => {
                // Write to log file if configured
                if !output_file.as_os_str().is_empty() {
                    if let Some(parent) = output_file.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Ok(mut file) = std::fs::File::create(output_file) {
                        let _ = file.write_all(&output.stdout);
                        let _ = file.write_all(&output.stderr);
                    }
                }

                let exit_code = output.status.code();
                if output.status.success() {
                    (StepState::Success, exit_code, None)
                } else {
                    (
                        StepState::Failed,
                        exit_code,
                        Some(format!("Exit code: {}", exit_code.unwrap_or(-1))),
                    )
                }
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
        let working_dir = self.context.working_dir();

        // Build command
        let builder = ClaudeCommandBuilder::from_agent_step(agent, &self.context);
        let args = builder.prompt(&expanded_prompt).build();

        let mut cmd = Command::new(&self.config.claude_command);
        cmd.args(&args);
        cmd.current_dir(working_dir);

        // Set environment variables
        for (key, value) in self.context.to_env_vars() {
            cmd.env(key, value);
        }

        // Configure output
        if output_file.as_os_str().is_empty() || !agent.print {
            cmd.stdout(Stdio::inherit());
            cmd.stderr(Stdio::inherit());
        } else {
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());
        }

        match cmd.spawn().and_then(|child| child.wait_with_output()) {
            Ok(output) => {
                // Write to log file if configured
                if !output_file.as_os_str().is_empty() && agent.print {
                    if let Some(parent) = output_file.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Ok(mut file) = std::fs::File::create(output_file) {
                        let _ = file.write_all(&output.stdout);
                        let _ = file.write_all(&output.stderr);
                    }
                }

                let exit_code = output.status.code();
                if output.status.success() {
                    (StepState::Success, exit_code, None)
                } else {
                    (
                        StepState::Failed,
                        exit_code,
                        Some(format!("Agent exited with code: {}", exit_code.unwrap_or(-1))),
                    )
                }
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
    fn test_evaluate_condition_equality() {
        let config = test_config();
        let context = test_context().with_prev_state("success");
        let executor = StepExecutor::new(&config, context);

        assert!(executor.evaluate_condition("'success' == 'success'"));
        assert!(!executor.evaluate_condition("'success' == 'failed'"));
        assert!(executor.evaluate_condition("'success' != 'failed'"));
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
