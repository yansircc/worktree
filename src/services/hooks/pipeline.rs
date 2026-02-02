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
//!
//! Key features:
//! - Automatic stream-json format for intermediate agents
//! - include_partial_messages enabled by default for real-time output
//! - Full Claude CLI parameter support
//! - Background execution support

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::error::{Result, WtError};
use crate::models::{Step, WtConfig};
use crate::services::claude::{shell_escape, ClaudeCommandBuilder};
use crate::services::hooks::context::ExecutionContext;

/// Pipeline execution options
#[derive(Debug, Clone, Default)]
pub struct PipelineOptions {
    /// Run pipeline in background
    pub background: bool,
    /// Enable verbose output
    pub verbose: bool,
    /// Pipeline name (for tracking)
    pub name: Option<String>,
}

/// Pipeline status for tracking
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PipelineStatus {
    pub id: String,
    pub name: Option<String>,
    pub command: String,
    pub pid: Option<u32>,
    pub start_time: String,
    pub status: PipelineState,
    pub output_file: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PipelineState {
    Running,
    Completed,
    Failed,
    Killed,
}

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
        self.execute_with_options(steps, PipelineOptions::default())
    }

    /// Execute a pipeline with options
    pub fn execute_with_options(&self, steps: &[Step], options: PipelineOptions) -> Result<()> {
        if steps.is_empty() {
            return Ok(());
        }

        // Extract agent steps only (pipeline only supports agent steps)
        let agents: Vec<_> = steps
            .iter()
            .filter(|s| matches!(s, Step::Agent { .. }))
            .collect();

        if agents.is_empty() {
            return Err(WtError::InvalidInput(
                "Pipeline must contain at least one agent step".to_string(),
            ));
        }

        // Build the pipeline command
        let pipeline_cmd = self.build_pipeline_command(&agents);

        if options.background {
            self.execute_background(&pipeline_cmd, &options)
        } else {
            self.execute_foreground(&pipeline_cmd, options.verbose)
        }
    }

    /// Build the pipeline shell command with full parameter support
    fn build_pipeline_command(&self, agents: &[&Step]) -> String {
        let mut parts = Vec::new();
        let total = agents.len();

        for (i, step) in agents.iter().enumerate() {
            if let Step::Agent { agent } = step {
                let expanded_prompt = self.context.expand(&agent.prompt);

                // Start with base command from AgentStep
                // Note: ClaudeCommandBuilder::from_agent_step handles most fields,
                // but we need to override I/O format for pipeline
                let mut builder = ClaudeCommandBuilder::new();

                // Always print mode for pipeline
                builder = builder.print();

                // Model
                builder = builder.model(&agent.model);

                // === System Prompt (with shell escaping for pipeline) ===
                if let Some(ref sp) = agent.system_prompt {
                    builder = builder.system_prompt(&shell_escape(&self.context.expand(sp)));
                }
                if let Some(ref spf) = agent.system_prompt_file {
                    builder = builder.system_prompt_file(&self.context.expand(spf));
                }
                if let Some(ref asp) = agent.append_system_prompt {
                    builder = builder.append_system_prompt(&shell_escape(&self.context.expand(asp)));
                }
                if let Some(ref aspf) = agent.append_system_prompt_file {
                    builder = builder.append_system_prompt_file(&self.context.expand(aspf));
                }

                // === Tools ===
                if !agent.tools.is_empty() {
                    builder = builder.tools(&agent.tools);
                }
                for tool in &agent.allowed_tools {
                    builder = builder.allowed_tool(tool);
                }
                for tool in &agent.disallowed_tools {
                    builder = builder.disallowed_tool(tool);
                }

                // === Permissions ===
                if agent.skip_permissions {
                    builder = builder.skip_permissions();
                }
                if agent.allow_skip_permissions {
                    builder = builder.allow_skip_permissions();
                }
                if let Some(ref mode) = agent.permission_mode {
                    builder = builder.permission_mode(mode);
                }
                if let Some(ref tool) = agent.permission_prompt_tool {
                    builder = builder.permission_prompt_tool(tool);
                }

                // === Limits ===
                if let Some(turns) = agent.max_turns {
                    builder = builder.max_turns(turns);
                }
                if let Some(budget) = agent.max_budget_usd {
                    builder = builder.max_budget_usd(budget);
                }

                // === Session ===
                if agent.continue_session {
                    builder = builder.continue_session();
                }
                if let Some(ref session) = agent.resume {
                    builder = builder.resume(session);
                }
                if let Some(ref sid) = agent.session_id {
                    builder = builder.session_id(sid);
                }
                if agent.fork_session {
                    builder = builder.fork_session();
                }
                if agent.no_session_persistence {
                    builder = builder.no_session_persistence();
                }

                // === I/O Format (automatic for pipeline) ===
                if i > 0 {
                    builder = builder.input_format("stream-json");
                }
                if i < total - 1 {
                    builder = builder.output_format("stream-json");
                }

                // include_partial_messages: default true for pipeline
                if agent.include_partial_messages || i < total - 1 {
                    builder = builder.include_partial_messages();
                }

                if let Some(ref schema) = agent.json_schema {
                    builder = builder.json_schema(&shell_escape(&self.context.expand(schema)));
                }

                // === Fallback Model ===
                if let Some(ref fallback) = agent.fallback_model {
                    builder = builder.fallback_model(fallback);
                }

                // === Subagents ===
                if let Some(ref agents_json) = agent.agents {
                    builder = builder.agents(&shell_escape(&agents_json.to_string()));
                }
                if let Some(ref agent_name) = agent.agent {
                    builder = builder.agent(agent_name);
                }

                // === Other ===
                for dir in &agent.add_dir {
                    builder = builder.add_dir(&self.context.expand(dir));
                }
                if let Some(ref mcp) = agent.mcp_config {
                    builder = builder.mcp_config(&self.context.expand(mcp));
                }
                if agent.strict_mcp_config {
                    builder = builder.strict_mcp_config();
                }
                if agent.verbose {
                    builder = builder.verbose();
                }
                if let Some(ref dbg) = agent.debug {
                    builder = builder.debug(dbg);
                }
                if let Some(ref s) = agent.settings {
                    builder = builder.settings(&self.context.expand(s));
                }
                if let Some(ref src) = agent.setting_sources {
                    builder = builder.setting_sources(src);
                }
                for dir in &agent.plugin_dir {
                    builder = builder.plugin_dir(&self.context.expand(dir));
                }
                for beta in &agent.betas {
                    builder = builder.beta(beta);
                }

                // === Browser/IDE ===
                match agent.chrome {
                    Some(true) => builder = builder.chrome(),
                    Some(false) => builder = builder.no_chrome(),
                    None => {}
                }
                if agent.ide {
                    builder = builder.ide();
                }
                if agent.disable_slash_commands {
                    builder = builder.disable_slash_commands();
                }

                // Prompt (always last, with shell escaping)
                let cmd = builder
                    .prompt_escaped(&expanded_prompt)
                    .build_command_string(&self.config.claude_command);

                parts.push(cmd);
            }
        }

        parts.join(" | ")
    }

    /// Execute pipeline in foreground
    fn execute_foreground(&self, command: &str, verbose: bool) -> Result<()> {
        let working_dir = self.context.working_dir();

        if verbose {
            eprintln!("[pipeline] Executing: {}", command);
        }

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

    /// Execute pipeline in background
    fn execute_background(&self, command: &str, options: &PipelineOptions) -> Result<()> {
        let working_dir = self.context.working_dir();
        let pipeline_id = format!("pipeline_{}", chrono::Utc::now().timestamp_millis());

        // Create pipelines tracking directory
        let pipelines_dir = PathBuf::from(&self.context.repo_root).join(".wt/pipelines");
        fs::create_dir_all(&pipelines_dir).map_err(|e| WtError::Io {
            operation: "create_dir".to_string(),
            path: pipelines_dir.display().to_string(),
            message: e.to_string(),
        })?;

        // Output file for background process
        let output_file = pipelines_dir.join(format!("{}.log", pipeline_id));
        let status_file = pipelines_dir.join(format!("{}.json", pipeline_id));

        // Build background command with output redirection
        let bg_command = format!(
            "({} > {} 2>&1; echo $? > {}.exit) &",
            command,
            output_file.display(),
            output_file.display()
        );

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(&bg_command);
        cmd.current_dir(working_dir);

        // Set environment variables
        for (key, value) in self.context.to_env_vars() {
            cmd.env(key, value);
        }

        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());

        let child = cmd.spawn().map_err(|e| WtError::Script {
            script: command.to_string(),
            message: format!("Failed to start background pipeline: {}", e),
        })?;

        // Save pipeline status
        let status = PipelineStatus {
            id: pipeline_id.clone(),
            name: options.name.clone(),
            command: command.to_string(),
            pid: Some(child.id()),
            start_time: chrono::Utc::now().to_rfc3339(),
            status: PipelineState::Running,
            output_file: Some(output_file.to_string_lossy().to_string()),
        };

        let status_json = serde_json::to_string_pretty(&status)
            .map_err(|e| WtError::ConfigRead(e.to_string()))?;

        let mut file = fs::File::create(&status_file).map_err(|e| WtError::Io {
            operation: "create".to_string(),
            path: status_file.display().to_string(),
            message: e.to_string(),
        })?;
        file.write_all(status_json.as_bytes()).map_err(|e| WtError::Io {
            operation: "write".to_string(),
            path: status_file.display().to_string(),
            message: e.to_string(),
        })?;

        eprintln!("[pipeline] Started in background: {}", pipeline_id);
        eprintln!("[pipeline] Output: {}", output_file.display());
        eprintln!("[pipeline] Status: {}", status_file.display());

        Ok(())
    }
}

/// List all pipeline statuses
pub fn list_pipelines(repo_root: &str) -> Result<Vec<PipelineStatus>> {
    super::pipeline_store::PipelineStore::new(repo_root).list()
}

/// Kill a running pipeline
pub fn kill_pipeline(repo_root: &str, pipeline_id: &str) -> Result<()> {
    let store = super::pipeline_store::PipelineStore::new(repo_root);
    let mut status = store.load(pipeline_id)?;

    if status.status != PipelineState::Running {
        return Err(WtError::InvalidInput(format!(
            "Pipeline '{}' is not running (status: {:?})",
            pipeline_id, status.status
        )));
    }

    if let Some(pid) = status.pid {
        kill_process(pid);
        status.status = PipelineState::Killed;
        store.save(&status)?;
        eprintln!("[pipeline] Killed: {}", pipeline_id);
    }

    Ok(())
}

fn kill_process(pid: u32) {
    let pid_str = pid.to_string();
    if Command::new("kill")
        .args(["-TERM", &format!("-{}", pid_str)])
        .output()
        .is_err()
    {
        let _ = Command::new("kill").args(["-TERM", &pid_str]).output();
    }
}

/// Clean up old pipeline records
pub fn cleanup_pipelines(repo_root: &str, max_age_hours: u64) -> Result<usize> {
    let store = super::pipeline_store::PipelineStore::new(repo_root);
    let cutoff = chrono::Utc::now() - chrono::Duration::hours(max_age_hours as i64);
    let mut removed = 0;

    for status in store.list()? {
        if status.status != PipelineState::Running {
            if let Ok(start) = chrono::DateTime::parse_from_rfc3339(&status.start_time) {
                if start < cutoff {
                    store.remove(&status.id);
                    removed += 1;
                }
            }
        }
    }

    Ok(removed)
}

/// Create an agent step for testing
#[cfg(test)]
fn make_agent_step(model: &str, prompt: &str) -> Step {
    use crate::models::AgentStep;
    Step::Agent {
        agent: AgentStep::new(prompt).with_model(model).with_print(),
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
    }

    #[test]
    fn test_build_pipeline_single_agent() {
        let config = test_config();
        let context = test_context();
        let executor = PipelineExecutor::new(&config, &context);

        let step = make_agent_step("haiku", "summarize changes");
        let agents = vec![&step];
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

        let step1 = make_agent_step("haiku", "list files");
        let step2 = make_agent_step("sonnet", "review code");
        let step3 = make_agent_step("haiku", "summarize");
        let agents = vec![&step1, &step2, &step3];
        let cmd = executor.build_pipeline_command(&agents);

        // Should have pipe separators
        assert!(cmd.contains(" | "));

        // Count pipe separators (should be 2 for 3 agents)
        let pipe_count = cmd.matches(" | ").count();
        assert_eq!(pipe_count, 2);

        // First agent: output-format stream-json, no input-format
        // Middle agent: both input-format and output-format stream-json
        // Last agent: input-format stream-json, no output-format
        assert!(cmd.contains("--output-format"));
        assert!(cmd.contains("--input-format"));
    }

    #[test]
    fn test_build_pipeline_with_variable_expansion() {
        let config = test_config();
        let context = test_context();
        let executor = PipelineExecutor::new(&config, &context);

        let step = make_agent_step("sonnet", "review @.wt/tasks/${task}.md");
        let agents = vec![&step];
        let cmd = executor.build_pipeline_command(&agents);

        // Variable should be expanded
        assert!(cmd.contains("auth"));
        assert!(!cmd.contains("${task}"));
    }

    #[test]
    fn test_build_pipeline_includes_partial_messages() {
        let config = test_config();
        let context = test_context();
        let executor = PipelineExecutor::new(&config, &context);

        let step1 = make_agent_step("haiku", "analyze");
        let step2 = make_agent_step("sonnet", "process");
        let agents = vec![&step1, &step2];
        let cmd = executor.build_pipeline_command(&agents);

        // All agents in pipeline should have include-partial-messages
        // (except possibly the last one if not explicitly set)
        assert!(cmd.contains("--include-partial-messages"));
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

    #[test]
    fn test_pipeline_options_default() {
        let options = PipelineOptions::default();
        assert!(!options.background);
        assert!(!options.verbose);
        assert!(options.name.is_none());
    }

    #[test]
    fn test_pipeline_state_serde() {
        let state = PipelineState::Running;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"running\"");

        let parsed: PipelineState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, PipelineState::Running);
    }
}
