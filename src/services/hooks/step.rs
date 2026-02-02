//! Step executor for hooks v2.
//!
//! Executes individual steps: script, agent, internal, condition.

use std::process::{Command, Stdio};

use crate::error::{Result, WtError};
use crate::models::{WtConfig, Step};
use crate::services::hooks::context::ExecutionContext;

/// Result of step execution
#[derive(Debug, Clone)]
pub struct StepResult {
    /// Whether the step succeeded
    pub success: bool,
    /// Output from the step (if captured)
    pub output: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl StepResult {
    pub fn ok() -> Self {
        Self {
            success: true,
            output: None,
            error: None,
        }
    }

    pub fn ok_with_output(output: String) -> Self {
        Self {
            success: true,
            output: Some(output),
            error: None,
        }
    }

    pub fn err(message: String) -> Self {
        Self {
            success: false,
            output: None,
            error: Some(message),
        }
    }
}

/// Parameters for agent execution
#[derive(Debug)]
struct AgentParams<'a> {
    interactive: bool,
    model: &'a str,
    prompt: &'a str,
    system_prompt: Option<&'a str>,
    system_prompt_file: Option<&'a str>,
    append_system_prompt: Option<&'a str>,
    append_system_prompt_file: Option<&'a str>,
    tools: &'a [String],
    allowed_tools: &'a [String],
    disallowed_tools: &'a [String],
    skip_permissions: bool,
    permission_mode: Option<&'a str>,
    max_turns: Option<u32>,
    max_budget_usd: Option<f64>,
    continue_session: bool,
    resume: Option<&'a str>,
    output_format: &'a str,
    input_format: Option<&'a str>,
    add_dir: &'a [String],
    mcp_config: Option<&'a str>,
    verbose: bool,
    window: Option<&'a str>,
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
            Step::Agent {
                interactive,
                model,
                prompt,
                system_prompt,
                system_prompt_file,
                append_system_prompt,
                append_system_prompt_file,
                tools,
                allowed_tools,
                disallowed_tools,
                skip_permissions,
                permission_mode,
                max_turns,
                max_budget_usd,
                continue_session,
                resume,
                output_format,
                input_format,
                add_dir,
                mcp_config,
                verbose,
                window,
            } => self.execute_agent(AgentParams {
                interactive: *interactive,
                model,
                prompt,
                system_prompt: system_prompt.as_deref(),
                system_prompt_file: system_prompt_file.as_deref(),
                append_system_prompt: append_system_prompt.as_deref(),
                append_system_prompt_file: append_system_prompt_file.as_deref(),
                tools,
                allowed_tools,
                disallowed_tools,
                skip_permissions: *skip_permissions,
                permission_mode: permission_mode.as_deref(),
                max_turns: *max_turns,
                max_budget_usd: *max_budget_usd,
                continue_session: *continue_session,
                resume: resume.as_deref(),
                output_format,
                input_format: input_format.as_deref(),
                add_dir,
                mcp_config: mcp_config.as_deref(),
                verbose: *verbose,
                window: window.as_deref(),
            }),
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
    fn execute_agent(&self, params: AgentParams) -> Result<StepResult> {
        let expanded_prompt = self.context.expand(params.prompt);
        let working_dir = self.context.working_dir();

        // Build claude command arguments
        let mut args = Vec::new();

        // Print mode (non-interactive) or REPL (interactive)
        if !params.interactive {
            args.push("-p".to_string());
        }

        // Model selection
        let model_arg = match params.model {
            "haiku" => "claude-haiku-4-20250514".to_string(),
            "sonnet" => "claude-sonnet-4-20250514".to_string(),
            "opus" => "claude-opus-4-20250514".to_string(),
            other => other.to_string(),
        };
        args.push("--model".to_string());
        args.push(model_arg);

        // === System Prompt ===
        if let Some(sp) = params.system_prompt {
            args.push("--system-prompt".to_string());
            args.push(self.context.expand(sp));
        }
        if let Some(spf) = params.system_prompt_file {
            args.push("--system-prompt-file".to_string());
            args.push(self.context.expand(spf));
        }
        if let Some(asp) = params.append_system_prompt {
            args.push("--append-system-prompt".to_string());
            args.push(self.context.expand(asp));
        }
        if let Some(aspf) = params.append_system_prompt_file {
            args.push("--append-system-prompt-file".to_string());
            args.push(self.context.expand(aspf));
        }

        // === Tools ===
        if !params.tools.is_empty() {
            args.push("--tools".to_string());
            args.push(params.tools.join(","));
        }
        for tool in params.allowed_tools {
            args.push("--allowedTools".to_string());
            args.push(tool.clone());
        }
        for tool in params.disallowed_tools {
            args.push("--disallowedTools".to_string());
            args.push(tool.clone());
        }

        // === Permissions ===
        if params.skip_permissions {
            args.push("--dangerously-skip-permissions".to_string());
        }
        if let Some(mode) = params.permission_mode {
            args.push("--permission-mode".to_string());
            args.push(mode.to_string());
        }

        // === Limits ===
        if let Some(turns) = params.max_turns {
            args.push("--max-turns".to_string());
            args.push(turns.to_string());
        }
        if let Some(budget) = params.max_budget_usd {
            args.push("--max-budget-usd".to_string());
            args.push(budget.to_string());
        }

        // === Session ===
        if params.continue_session {
            args.push("--continue".to_string());
        }
        if let Some(session_id) = params.resume {
            args.push("--resume".to_string());
            args.push(session_id.to_string());
        }

        // === Input/Output ===
        if !params.interactive && params.output_format != "text" {
            args.push("--output-format".to_string());
            args.push(params.output_format.to_string());
        }
        if let Some(input_fmt) = params.input_format {
            args.push("--input-format".to_string());
            args.push(input_fmt.to_string());
        }

        // === Other ===
        for dir in params.add_dir {
            args.push("--add-dir".to_string());
            args.push(self.context.expand(dir));
        }
        if let Some(mcp) = params.mcp_config {
            args.push("--mcp-config".to_string());
            args.push(self.context.expand(mcp));
        }
        if params.verbose {
            args.push("--verbose".to_string());
        }

        // Prompt (for non-interactive mode)
        if !params.interactive {
            args.push(expanded_prompt.clone());
        }

        // For interactive mode in a window
        if params.interactive {
            return self.execute_agent_in_window(&args, &expanded_prompt, params.window);
        }

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

/// Escape a string for shell
fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
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
        assert!(r.output.is_none());
        assert!(r.error.is_none());
    }

    #[test]
    fn test_step_result_err() {
        let r = StepResult::err("failed".to_string());
        assert!(!r.success);
        assert_eq!(r.error, Some("failed".to_string()));
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
