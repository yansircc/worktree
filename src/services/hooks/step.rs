//! Step executor for hooks v2.
//!
//! Executes individual steps: script, agent, internal, condition.

use std::process::{Command, Stdio};

use crate::error::{Result, WtError};
use crate::models::{AgentStep, Step, WtConfig};
use crate::services::claude::{shell_escape, ClaudeCommandBuilder};
use crate::services::hooks::context::ExecutionContext;

/// Result of step execution
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields reserved for future step output capture
pub struct StepResult {
    /// Whether the step succeeded
    success: bool,
    /// Output from the step (if captured)
    output: Option<String>,
    /// Error message (if failed)
    error: Option<String>,
}

impl StepResult {
    pub fn ok() -> Self {
        Self {
            success: true,
            output: None,
            error: None,
        }
    }
}

/// Step executor
pub struct StepExecutor<'a> {
    config: &'a WtConfig,
    context: &'a ExecutionContext,
}

impl<'a> StepExecutor<'a> {
    pub fn new(config: &'a WtConfig, context: &'a ExecutionContext) -> Self {
        Self { config, context }
    }

    /// Execute a step
    pub fn execute(&self, step: &Step) -> Result<StepResult> {
        match step {
            Step::Script { run, on_error } => self.execute_script(run, on_error.as_deref()),
            Step::Agent { agent } => self.execute_agent(agent),
            Step::Internal { run, on_conflict } => {
                self.execute_internal(run, on_conflict.as_deref())
            }
            Step::Condition { if_, then, else_ } => {
                self.execute_condition(if_, then, else_.as_deref())
            }
        }
    }

    /// Execute a shell script
    fn execute_script(&self, script: &str, on_error: Option<&Step>) -> Result<StepResult> {
        let expanded = self.context.expand(script);
        let working_dir = self.context.working_dir();

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(&expanded);
        cmd.current_dir(working_dir);

        // Set environment variables
        for (key, value) in self.context.to_env_vars() {
            cmd.env(key, value);
        }

        // Stream output to terminal
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let status = cmd
            .spawn()
            .and_then(|mut child| child.wait())
            .map_err(|e| WtError::Script {
                script: expanded.clone(),
                message: format!("Failed to execute: {}", e),
            })?;

        if status.success() {
            Ok(StepResult::ok())
        } else {
            // Try on_error handler if defined
            if let Some(error_step) = on_error {
                return self.execute(error_step);
            }

            Err(WtError::Script {
                script: expanded,
                message: format!("Exit code: {}", status.code().unwrap_or(-1)),
            })
        }
    }

    /// Execute a Claude agent
    fn execute_agent(&self, agent: &AgentStep) -> Result<StepResult> {
        let expanded_prompt = self.context.expand(&agent.prompt);
        let working_dir = self.context.working_dir();

        // Build command using ClaudeCommandBuilder
        let builder = ClaudeCommandBuilder::from_agent_step(agent, self.context);

        // For REPL mode (non-print), run in a multiplexer window
        if !agent.print {
            let args = builder.build();
            return self.execute_agent_in_window(&args, &expanded_prompt, agent.window.as_deref());
        }

        // For print mode, add prompt and execute
        let args = builder.prompt(&expanded_prompt).build();

        // Execute non-interactive agent
        let mut cmd = Command::new(&self.config.claude_command);
        cmd.args(&args);
        cmd.current_dir(working_dir);

        // Set environment variables
        for (key, value) in self.context.to_env_vars() {
            cmd.env(key, value);
        }

        // Stream output
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let status = cmd
            .spawn()
            .and_then(|mut child| child.wait())
            .map_err(|e| WtError::Script {
                script: format!("{} {}", self.config.claude_command, args.join(" ")),
                message: format!("Failed to execute agent: {}", e),
            })?;

        if status.success() {
            Ok(StepResult::ok())
        } else {
            Err(WtError::Script {
                script: "claude agent".to_string(),
                message: format!("Agent exited with code: {}", status.code().unwrap_or(-1)),
            })
        }
    }

    /// Execute agent in a multiplexer window (interactive mode)
    fn execute_agent_in_window(
        &self,
        args: &[String],
        prompt: &str,
        _window: Option<&str>,
    ) -> Result<StepResult> {
        // Build the full command to run in the window
        let claude_cmd = format!(
            "{} {} {}",
            self.config.claude_command,
            args.join(" "),
            shell_escape(prompt)
        );

        // Use wt internal mux:send-keys to send the command
        // This is a placeholder - actual implementation depends on multiplexer state
        let internal_args = vec![
            "mux:send-keys".to_string(),
            "--session".to_string(),
            self.context.session.clone(),
            "--window".to_string(),
            self.context.window.clone(),
            "--".to_string(),
            claude_cmd,
        ];

        self.run_internal_command(&internal_args)
    }

    /// Execute an internal wt command
    fn execute_internal(&self, operation: &str, on_conflict: Option<&Step>) -> Result<StepResult> {
        let expanded = self.context.expand(operation);

        // Parse operation and args
        let parts: Vec<&str> = expanded.split_whitespace().collect();
        if parts.is_empty() {
            return Err(WtError::InvalidInput(
                "Empty internal operation".to_string(),
            ));
        }

        let op = parts[0].to_string();
        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

        // Execute via wt internal
        let result = crate::commands::internal::execute(op.clone(), args);

        match result {
            Ok(()) => Ok(StepResult::ok()),
            Err(e) => {
                // Check if this is a conflict error and we have a handler
                let is_conflict = matches!(&e, WtError::Git(msg) if msg.contains("conflict"));

                if is_conflict {
                    if let Some(conflict_step) = on_conflict {
                        return self.execute(conflict_step);
                    }
                }

                Err(e)
            }
        }
    }

    /// Run an internal command with arguments
    fn run_internal_command(&self, args: &[String]) -> Result<StepResult> {
        if args.is_empty() {
            return Err(WtError::InvalidInput(
                "Empty internal command".to_string(),
            ));
        }

        let op = args[0].clone();
        let cmd_args = args[1..].to_vec();

        crate::commands::internal::execute(op, cmd_args)?;
        Ok(StepResult::ok())
    }

    /// Execute a condition step
    fn execute_condition(
        &self,
        condition: &str,
        then_step: &Step,
        else_step: Option<&Step>,
    ) -> Result<StepResult> {
        let expanded = self.context.expand(condition);

        // Execute condition as a shell command
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(&expanded);
        cmd.current_dir(self.context.working_dir());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());

        let status = cmd.status().map_err(|e| WtError::Script {
            script: expanded.clone(),
            message: format!("Failed to evaluate condition: {}", e),
        })?;

        if status.success() {
            self.execute(then_step)
        } else if let Some(else_s) = else_step {
            self.execute(else_s)
        } else {
            // Condition false, no else branch - skip silently
            Ok(StepResult::ok())
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
        // Use current directory for tests
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string());
        ExecutionContext::new("auth", "wt/auth", &cwd, &cwd)
            .with_session("wt")
            .with_window("auth")
            .with_phase("developing")
    }

    #[test]
    fn test_step_result_ok() {
        let r = StepResult::ok();
        assert!(r.success);
    }

    #[test]
    fn test_execute_script_success() {
        let config = test_config();
        let context = test_context();
        let executor = StepExecutor::new(&config, &context);

        let step = Step::Script {
            run: "true".to_string(),
            on_error: None,
        };

        let result = executor.execute(&step);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_script_failure() {
        let config = test_config();
        let context = test_context();
        let executor = StepExecutor::new(&config, &context);

        let step = Step::Script {
            run: "exit 1".to_string(),
            on_error: None,
        };

        let result = executor.execute(&step);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_script_with_variables() {
        let config = test_config();
        let context = test_context();
        let executor = StepExecutor::new(&config, &context);

        let step = Step::Script {
            run: "test '${task}' = 'auth'".to_string(),
            on_error: None,
        };

        let result = executor.execute(&step);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_script_on_error() {
        let config = test_config();
        let context = test_context();
        let executor = StepExecutor::new(&config, &context);

        let step = Step::Script {
            run: "exit 1".to_string(),
            on_error: Some(Box::new(Step::Script {
                run: "true".to_string(), // Recovery succeeds
                on_error: None,
            })),
        };

        let result = executor.execute(&step);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_condition_true() {
        let config = test_config();
        let context = test_context();
        let executor = StepExecutor::new(&config, &context);

        let step = Step::Condition {
            if_: "true".to_string(),
            then: Box::new(Step::Script {
                run: "true".to_string(),
                on_error: None,
            }),
            else_: Some(Box::new(Step::Script {
                run: "exit 1".to_string(),
                on_error: None,
            })),
        };

        let result = executor.execute(&step);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_condition_false() {
        let config = test_config();
        let context = test_context();
        let executor = StepExecutor::new(&config, &context);

        let step = Step::Condition {
            if_: "false".to_string(),
            then: Box::new(Step::Script {
                run: "exit 1".to_string(), // Should not run
                on_error: None,
            }),
            else_: Some(Box::new(Step::Script {
                run: "true".to_string(),
                on_error: None,
            })),
        };

        let result = executor.execute(&step);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_condition_false_no_else() {
        let config = test_config();
        let context = test_context();
        let executor = StepExecutor::new(&config, &context);

        let step = Step::Condition {
            if_: "false".to_string(),
            then: Box::new(Step::Script {
                run: "exit 1".to_string(),
                on_error: None,
            }),
            else_: None,
        };

        // Should succeed (skip silently)
        let result = executor.execute(&step);
        assert!(result.is_ok());
    }

    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape("hello"), "'hello'");
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }
}
