//! Pipeline executor for hooks v2.
//!
//! Executes multiple agents chained via stream-json format.
//!
//! Pipeline format:
//! ```bash
//! claude -p --output-format stream-json "prompt1" | \
//! claude -p --input-format stream-json --output-format stream-json "prompt2" | \
//! claude -p --input-format stream-json "prompt3"
//! ```

use std::process::{Command, Stdio};

use crate::error::{Result, WtError};
use crate::models::{WtConfig, Step};
use crate::services::hooks::context::ExecutionContext;

/// Pipeline executor
pub struct PipelineExecutor<'a> {
    config: &'a WtConfig,
    context: &'a ExecutionContext,
}

impl<'a> PipelineExecutor<'a> {
    pub fn new(config: &'a WtConfig, context: &'a ExecutionContext) -> Self {
        Self { config, context }
    }

    /// Execute a pipeline of agents
    pub fn execute(&self, steps: &[Step]) -> Result<()> {
        if steps.is_empty() {
            return Ok(());
        }

        // Extract agent steps only (pipeline only supports agent steps)
        let agents: Vec<_> = steps
            .iter()
            .filter_map(|s| {
                if let Step::Agent {
                    model,
                    prompt,
                    tools,
                    allowed_tools,
                    skip_permissions,
                    ..
                } = s
                {
                    Some((model, prompt, tools, allowed_tools, *skip_permissions))
                } else {
                    None
                }
            })
            .collect();

        if agents.is_empty() {
            return Err(WtError::InvalidInput(
                "Pipeline must contain at least one agent step".to_string(),
            ));
        }

        // Build the pipeline command
        let pipeline_cmd = self.build_pipeline_command(&agents);

        // Execute via shell
        self.execute_pipeline(&pipeline_cmd)
    }

    /// Build the pipeline shell command
    fn build_pipeline_command(
        &self,
        agents: &[(&String, &String, &Vec<String>, &Vec<String>, bool)],
    ) -> String {
        let mut parts = Vec::new();

        for (i, (model, prompt, tools, allowed_tools, skip_permissions)) in
            agents.iter().enumerate()
        {
            let expanded_prompt = self.context.expand(prompt);
            let mut args = Vec::new();

            // Always print mode for pipeline
            args.push("-p".to_string());

            // Model
            let model_arg = match model.as_str() {
                "haiku" => "claude-haiku-4-20250514".to_string(),
                "sonnet" => "claude-sonnet-4-20250514".to_string(),
                "opus" => "claude-opus-4-20250514".to_string(),
                other => other.to_string(),
            };
            args.push("--model".to_string());
            args.push(model_arg);

            // Input format (not for first agent)
            if i > 0 {
                args.push("--input-format".to_string());
                args.push("stream-json".to_string());
            }

            // Output format (not for last agent)
            if i < agents.len() - 1 {
                args.push("--output-format".to_string());
                args.push("stream-json".to_string());
            }

            // Tools
            if !tools.is_empty() {
                args.push("--tools".to_string());
                args.push(tools.join(","));
            }

            // Allowed tools
            for tool in *allowed_tools {
                args.push("--allowedTools".to_string());
                args.push(tool.clone());
            }

            // Skip permissions
            if *skip_permissions {
                args.push("--dangerously-skip-permissions".to_string());
            }

            // Prompt
            args.push(shell_escape(&expanded_prompt));

            parts.push(format!("{} {}", self.config.claude_command, args.join(" ")));
        }

        parts.join(" | ")
    }

    /// Execute the pipeline command
    fn execute_pipeline(&self, command: &str) -> Result<()> {
        let working_dir = self.context.working_dir();

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);
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
                script: command.to_string(),
                message: format!("Failed to execute pipeline: {}", e),
            })?;

        if status.success() {
            Ok(())
        } else {
            Err(WtError::Script {
                script: "pipeline".to_string(),
                message: format!("Pipeline exited with code: {}", status.code().unwrap_or(-1)),
            })
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
    }

    #[test]
    fn test_build_pipeline_single_agent() {
        let config = test_config();
        let context = test_context();
        let executor = PipelineExecutor::new(&config, &context);

        let model = "haiku".to_string();
        let prompt = "summarize changes".to_string();
        let tools: Vec<String> = vec![];
        let allowed: Vec<String> = vec![];

        let agents = vec![(&model, &prompt, &tools, &allowed, false)];
        let cmd = executor.build_pipeline_command(&agents);

        // Single agent: no input-format, no output-format
        assert!(cmd.contains("claude"));
        assert!(cmd.contains("-p"));
        assert!(cmd.contains("--model"));
        assert!(!cmd.contains("--input-format"));
        assert!(!cmd.contains("--output-format"));
    }

    #[test]
    fn test_build_pipeline_multi_agent() {
        let config = test_config();
        let context = test_context();
        let executor = PipelineExecutor::new(&config, &context);

        let model1 = "haiku".to_string();
        let prompt1 = "list files".to_string();
        let model2 = "sonnet".to_string();
        let prompt2 = "review code".to_string();
        let model3 = "haiku".to_string();
        let prompt3 = "summarize".to_string();
        let tools: Vec<String> = vec![];
        let allowed: Vec<String> = vec![];

        let agents = vec![
            (&model1, &prompt1, &tools, &allowed, false),
            (&model2, &prompt2, &tools, &allowed, false),
            (&model3, &prompt3, &tools, &allowed, false),
        ];
        let cmd = executor.build_pipeline_command(&agents);

        // Should have pipe separators
        assert!(cmd.contains(" | "));

        // Count pipe separators (should be 2 for 3 agents)
        let pipe_count = cmd.matches(" | ").count();
        assert_eq!(pipe_count, 2);
    }

    #[test]
    fn test_build_pipeline_with_variable_expansion() {
        let config = test_config();
        let context = test_context();
        let executor = PipelineExecutor::new(&config, &context);

        let model = "sonnet".to_string();
        let prompt = "review @.wt/tasks/${task}.md".to_string();
        let tools: Vec<String> = vec![];
        let allowed: Vec<String> = vec![];

        let agents = vec![(&model, &prompt, &tools, &allowed, false)];
        let cmd = executor.build_pipeline_command(&agents);

        // Variable should be expanded
        assert!(cmd.contains("auth"));
        assert!(!cmd.contains("${task}"));
    }

    #[test]
    fn test_execute_empty_pipeline() {
        let config = test_config();
        let context = test_context();
        let executor = PipelineExecutor::new(&config, &context);

        let result = executor.execute(&[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_non_agent_steps_error() {
        let config = test_config();
        let context = test_context();
        let executor = PipelineExecutor::new(&config, &context);

        let steps = vec![Step::Script {
            run: "echo test".to_string(),
            on_error: None,
        }];

        let result = executor.execute(&steps);
        assert!(result.is_err());
    }
}
