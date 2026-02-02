//! Claude CLI command builder.
//!
//! Provides a unified interface for building Claude CLI command arguments.

use crate::models::AgentStep;
use crate::services::hooks::context::ExecutionContext;

/// Builder for Claude CLI command arguments
pub struct ClaudeCommandBuilder {
    args: Vec<String>,
}

impl ClaudeCommandBuilder {
    pub fn new() -> Self {
        Self { args: Vec::new() }
    }

    /// Build from AgentStep with context for variable expansion
    pub fn from_agent_step(step: &AgentStep, ctx: &ExecutionContext) -> Self {
        let mut builder = Self::new();

        // === Basic ===
        if step.print {
            builder = builder.print();
        }
        builder = builder.model(&step.model);

        // === System Prompt ===
        if let Some(ref sp) = step.system_prompt {
            builder = builder.system_prompt(&ctx.expand(sp));
        }
        if let Some(ref spf) = step.system_prompt_file {
            builder = builder.system_prompt_file(&ctx.expand(spf));
        }
        if let Some(ref asp) = step.append_system_prompt {
            builder = builder.append_system_prompt(&ctx.expand(asp));
        }
        if let Some(ref aspf) = step.append_system_prompt_file {
            builder = builder.append_system_prompt_file(&ctx.expand(aspf));
        }

        // === Tools ===
        if !step.tools.is_empty() {
            builder = builder.tools(&step.tools);
        }
        for tool in &step.allowed_tools {
            builder = builder.allowed_tool(tool);
        }
        for tool in &step.disallowed_tools {
            builder = builder.disallowed_tool(tool);
        }

        // === Permissions ===
        if step.skip_permissions {
            builder = builder.skip_permissions();
        }
        if step.allow_skip_permissions {
            builder = builder.allow_skip_permissions();
        }
        if let Some(ref mode) = step.permission_mode {
            builder = builder.permission_mode(mode);
        }
        if let Some(ref tool) = step.permission_prompt_tool {
            builder = builder.permission_prompt_tool(tool);
        }

        // === Limits ===
        if let Some(turns) = step.max_turns {
            builder = builder.max_turns(turns);
        }
        if let Some(budget) = step.max_budget_usd {
            builder = builder.max_budget_usd(budget);
        }

        // === Session ===
        if step.continue_session {
            builder = builder.continue_session();
        }
        if let Some(ref session) = step.resume {
            builder = builder.resume(session);
        }
        if let Some(ref sid) = step.session_id {
            builder = builder.session_id(sid);
        }
        if step.fork_session {
            builder = builder.fork_session();
        }
        if step.no_session_persistence {
            builder = builder.no_session_persistence();
        }

        // === I/O ===
        // output_format is handled separately for print mode
        if step.print && step.output_format != "text" {
            builder = builder.output_format(&step.output_format);
        }
        if let Some(ref input_fmt) = step.input_format {
            builder = builder.input_format(input_fmt);
        }
        if step.include_partial_messages {
            builder = builder.include_partial_messages();
        }
        if let Some(ref schema) = step.json_schema {
            builder = builder.json_schema(&ctx.expand(schema));
        }

        // === Model (fallback) ===
        if let Some(ref fallback) = step.fallback_model {
            builder = builder.fallback_model(fallback);
        }

        // === Subagents ===
        if let Some(ref agents_def) = step.agents {
            builder = builder.agents(&agents_def.to_string());
        }
        if let Some(ref agent_name) = step.agent {
            builder = builder.agent(agent_name);
        }

        // === Other ===
        for dir in &step.add_dir {
            builder = builder.add_dir(&ctx.expand(dir));
        }
        if let Some(ref mcp) = step.mcp_config {
            builder = builder.mcp_config(&ctx.expand(mcp));
        }
        if step.strict_mcp_config {
            builder = builder.strict_mcp_config();
        }
        if step.verbose {
            builder = builder.verbose();
        }
        if let Some(ref dbg) = step.debug {
            builder = builder.debug(dbg);
        }
        if let Some(ref s) = step.settings {
            builder = builder.settings(&ctx.expand(s));
        }
        if let Some(ref src) = step.setting_sources {
            builder = builder.setting_sources(src);
        }
        for dir in &step.plugin_dir {
            builder = builder.plugin_dir(&ctx.expand(dir));
        }
        for beta in &step.betas {
            builder = builder.beta(beta);
        }

        // === Browser/IDE ===
        match step.chrome {
            Some(true) => builder = builder.chrome(),
            Some(false) => builder = builder.no_chrome(),
            None => {}
        }
        if step.ide {
            builder = builder.ide();
        }
        if step.disable_slash_commands {
            builder = builder.disable_slash_commands();
        }

        builder
    }

    // === Basic ===
    pub fn print(mut self) -> Self {
        self.args.push("-p".to_string());
        self
    }

    pub fn model(mut self, m: &str) -> Self {
        let model = match m {
            "haiku" => "claude-haiku-4-20250514",
            "sonnet" => "claude-sonnet-4-20250514",
            "opus" => "claude-opus-4-20250514",
            other => other,
        };
        self.args.extend(["--model".into(), model.into()]);
        self
    }

    // === System Prompt ===
    pub fn system_prompt(mut self, sp: &str) -> Self {
        self.args.extend(["--system-prompt".into(), sp.into()]);
        self
    }

    pub fn system_prompt_file(mut self, path: &str) -> Self {
        self.args
            .extend(["--system-prompt-file".into(), path.into()]);
        self
    }

    pub fn append_system_prompt(mut self, asp: &str) -> Self {
        self.args
            .extend(["--append-system-prompt".into(), asp.into()]);
        self
    }

    pub fn append_system_prompt_file(mut self, path: &str) -> Self {
        self.args
            .extend(["--append-system-prompt-file".into(), path.into()]);
        self
    }

    // === Tools ===
    pub fn tools(mut self, tools: &[String]) -> Self {
        self.args.extend(["--tools".into(), tools.join(",")]);
        self
    }

    pub fn allowed_tool(mut self, tool: &str) -> Self {
        self.args.extend(["--allowedTools".into(), tool.into()]);
        self
    }

    pub fn disallowed_tool(mut self, tool: &str) -> Self {
        self.args.extend(["--disallowedTools".into(), tool.into()]);
        self
    }

    // === Permissions ===
    pub fn skip_permissions(mut self) -> Self {
        self.args.push("--dangerously-skip-permissions".into());
        self
    }

    pub fn allow_skip_permissions(mut self) -> Self {
        self.args
            .push("--allow-dangerously-skip-permissions".into());
        self
    }

    pub fn permission_mode(mut self, mode: &str) -> Self {
        self.args.extend(["--permission-mode".into(), mode.into()]);
        self
    }

    pub fn permission_prompt_tool(mut self, tool: &str) -> Self {
        self.args
            .extend(["--permission-prompt-tool".into(), tool.into()]);
        self
    }

    // === Limits ===
    pub fn max_turns(mut self, turns: u32) -> Self {
        self.args
            .extend(["--max-turns".into(), turns.to_string()]);
        self
    }

    pub fn max_budget_usd(mut self, budget: f64) -> Self {
        self.args
            .extend(["--max-budget-usd".into(), budget.to_string()]);
        self
    }

    // === Session ===
    pub fn continue_session(mut self) -> Self {
        self.args.push("--continue".into());
        self
    }

    pub fn resume(mut self, session: &str) -> Self {
        self.args.extend(["--resume".into(), session.into()]);
        self
    }

    pub fn session_id(mut self, sid: &str) -> Self {
        self.args.extend(["--session-id".into(), sid.into()]);
        self
    }

    pub fn fork_session(mut self) -> Self {
        self.args.push("--fork-session".into());
        self
    }

    pub fn no_session_persistence(mut self) -> Self {
        self.args.push("--no-session-persistence".into());
        self
    }

    // === I/O ===
    pub fn output_format(mut self, fmt: &str) -> Self {
        self.args.extend(["--output-format".into(), fmt.into()]);
        self
    }

    pub fn input_format(mut self, fmt: &str) -> Self {
        self.args.extend(["--input-format".into(), fmt.into()]);
        self
    }

    pub fn include_partial_messages(mut self) -> Self {
        self.args.push("--include-partial-messages".into());
        self
    }

    pub fn json_schema(mut self, schema: &str) -> Self {
        self.args.extend(["--json-schema".into(), schema.into()]);
        self
    }

    // === Model (fallback) ===
    pub fn fallback_model(mut self, model: &str) -> Self {
        self.args.extend(["--fallback-model".into(), model.into()]);
        self
    }

    // === Subagents ===
    pub fn agents(mut self, agents_json: &str) -> Self {
        self.args.extend(["--agents".into(), agents_json.into()]);
        self
    }

    pub fn agent(mut self, name: &str) -> Self {
        self.args.extend(["--agent".into(), name.into()]);
        self
    }

    // === Other ===
    pub fn add_dir(mut self, dir: &str) -> Self {
        self.args.extend(["--add-dir".into(), dir.into()]);
        self
    }

    pub fn mcp_config(mut self, config: &str) -> Self {
        self.args.extend(["--mcp-config".into(), config.into()]);
        self
    }

    pub fn strict_mcp_config(mut self) -> Self {
        self.args.push("--strict-mcp-config".into());
        self
    }

    pub fn verbose(mut self) -> Self {
        self.args.push("--verbose".into());
        self
    }

    pub fn debug(mut self, filter: &str) -> Self {
        self.args.extend(["--debug".into(), filter.into()]);
        self
    }

    pub fn settings(mut self, path: &str) -> Self {
        self.args.extend(["--settings".into(), path.into()]);
        self
    }

    pub fn setting_sources(mut self, sources: &str) -> Self {
        self.args.extend(["--setting-sources".into(), sources.into()]);
        self
    }

    pub fn plugin_dir(mut self, dir: &str) -> Self {
        self.args.extend(["--plugin-dir".into(), dir.into()]);
        self
    }

    pub fn beta(mut self, beta: &str) -> Self {
        self.args.extend(["--betas".into(), beta.into()]);
        self
    }

    // === Browser/IDE ===
    pub fn chrome(mut self) -> Self {
        self.args.push("--chrome".into());
        self
    }

    pub fn no_chrome(mut self) -> Self {
        self.args.push("--no-chrome".into());
        self
    }

    pub fn ide(mut self) -> Self {
        self.args.push("--ide".into());
        self
    }

    pub fn disable_slash_commands(mut self) -> Self {
        self.args.push("--disable-slash-commands".into());
        self
    }

    // === Prompt ===
    pub fn prompt(mut self, p: &str) -> Self {
        self.args.push(p.into());
        self
    }

    pub fn prompt_escaped(mut self, p: &str) -> Self {
        self.args.push(shell_escape(p));
        self
    }

    // === Build ===
    pub fn build(self) -> Vec<String> {
        self.args
    }

    pub fn build_command_string(self, claude_cmd: &str) -> String {
        format!("{} {}", claude_cmd, self.args.join(" "))
    }
}

impl Default for ClaudeCommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Escape a string for shell
pub fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_builder_basic() {
        let args = ClaudeCommandBuilder::new()
            .print()
            .model("sonnet")
            .prompt("Hello")
            .build();

        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"claude-sonnet-4-20250514".to_string()));
        assert!(args.contains(&"Hello".to_string()));
    }

    #[test]
    fn test_builder_model_aliases() {
        let haiku = ClaudeCommandBuilder::new().model("haiku").build();
        assert!(haiku.contains(&"claude-haiku-4-20250514".to_string()));

        let opus = ClaudeCommandBuilder::new().model("opus").build();
        assert!(opus.contains(&"claude-opus-4-20250514".to_string()));

        let custom = ClaudeCommandBuilder::new()
            .model("custom-model-id")
            .build();
        assert!(custom.contains(&"custom-model-id".to_string()));
    }

    #[test]
    fn test_builder_from_agent_step() {
        let step = AgentStep::new("Test prompt")
            .with_model("haiku")
            .with_print()
            .with_max_turns(5);

        let ctx = test_context();
        let args = ClaudeCommandBuilder::from_agent_step(&step, &ctx).build();

        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"--max-turns".to_string()));
        assert!(args.contains(&"5".to_string()));
    }

    #[test]
    fn test_builder_command_string() {
        let cmd = ClaudeCommandBuilder::new()
            .print()
            .model("sonnet")
            .prompt_escaped("Hello world")
            .build_command_string("claude");

        assert!(cmd.starts_with("claude "));
        assert!(cmd.contains("-p"));
        assert!(cmd.contains("--model"));
    }

    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape("hello"), "'hello'");
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
        assert_eq!(shell_escape("hello world"), "'hello world'");
    }
}
