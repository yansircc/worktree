//! Command execution utilities for git and tmux operations.

use std::path::Path;
use std::process::{Command, Output, Stdio};

use crate::error::{Result, WtError};

/// A helper for running external commands with consistent error handling.
pub struct CommandRunner {
    program: &'static str,
    error_mapper: fn(String) -> WtError,
    cwd: Option<String>,
    /// Whether to redirect stdin from null (prevents blocking on interactive prompts)
    null_stdin: bool,
}

impl CommandRunner {
    /// Create a new command runner with a custom program.
    pub fn new(program: &'static str) -> Self {
        Self {
            program,
            error_mapper: WtError::Git,
            cwd: None,
            null_stdin: false,
        }
    }

    /// Create a runner for git commands.
    pub fn git() -> Self {
        Self {
            program: "git",
            error_mapper: WtError::Git,
            cwd: None,
            null_stdin: false,
        }
    }

    /// Create a runner for tmux commands.
    pub fn tmux() -> Self {
        Self {
            program: "tmux",
            error_mapper: WtError::Tmux,
            cwd: None,
            null_stdin: false,
        }
    }

    /// Create a runner for zellij commands.
    /// Note: zellij action commands can block waiting for stdin, so we redirect from null.
    pub fn zellij() -> Self {
        Self {
            program: "zellij",
            error_mapper: WtError::Zellij,
            cwd: None,
            null_stdin: true,
        }
    }

    /// Set the working directory for the command.
    pub fn current_dir(mut self, dir: &str) -> Self {
        self.cwd = Some(dir.to_string());
        self
    }

    /// Run a command and check for success.
    pub fn run(&self, args: &[&str]) -> Result<()> {
        let output = self.execute(args)?;
        self.check_status(output)
    }

    /// Run a command and return stdout as a string.
    pub fn output(&self, args: &[&str]) -> Result<String> {
        let output = self.execute(args)?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err((self.error_mapper)(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    /// Check if a command succeeds without returning an error.
    pub fn success(&self, args: &[&str]) -> bool {
        self.build_command(args)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn build_command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(self.program);
        cmd.args(args);
        if let Some(ref cwd) = self.cwd {
            cmd.current_dir(Path::new(cwd));
        }
        if self.null_stdin {
            cmd.stdin(Stdio::null());
        }
        cmd
    }

    fn execute(&self, args: &[&str]) -> Result<Output> {
        self.build_command(args)
            .output()
            .map_err(|e| (self.error_mapper)(e.to_string()))
    }

    fn check_status(&self, output: Output) -> Result<()> {
        if output.status.success() {
            Ok(())
        } else {
            Err((self.error_mapper)(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_runner_success() {
        let runner = CommandRunner::git();
        assert!(runner.success(&["--version"]));
    }

    #[test]
    fn test_git_runner_failure() {
        let runner = CommandRunner::git();
        let result = runner.run(&["invalid-command-that-does-not-exist"]);
        assert!(result.is_err());
    }
}
