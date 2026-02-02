//! Tmux backend implementation.

use crate::error::Result;
use crate::services::command::CommandRunner;

use super::Multiplexer;

/// Tmux multiplexer backend
pub struct TmuxBackend;

impl TmuxBackend {
    pub fn new() -> Self {
        Self
    }

    fn runner(&self) -> CommandRunner {
        CommandRunner::tmux()
    }
}

impl Default for TmuxBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Multiplexer for TmuxBackend {
    fn is_available(&self) -> bool {
        self.runner().success(&["-V"])
    }

    fn session_exists(&self, session: &str) -> bool {
        self.runner().success(&["has-session", "-t", session])
    }

    fn create_session(&self, session: &str) -> Result<()> {
        self.runner().run(&["new-session", "-d", "-s", session])
    }

    fn create_window(&self, session: &str, window: &str, cwd: &str, command: &str) -> Result<()> {
        let target = format!("{}:", session);

        // Create window (starts interactive shell)
        self.runner()
            .run(&["new-window", "-t", &target, "-n", window, "-c", cwd])?;

        // Send command via send-keys if provided
        if !command.is_empty() {
            let window_target = format!("{}:{}", session, window);
            // Use -l (literal) to ensure special characters are sent correctly
            self.runner()
                .run(&["send-keys", "-t", &window_target, "-l", command])?;
            // Send Enter key separately
            self.runner()
                .run(&["send-keys", "-t", &window_target, "Enter"])?;
        }

        Ok(())
    }

    fn window_exists(&self, session: &str, window: &str) -> bool {
        let target = format!("{}:{}", session, window);
        self.runner().success(&["select-window", "-t", &target])
    }

    fn kill_window(&self, session: &str, window: &str) -> Result<()> {
        let target = format!("{}:{}", session, window);
        self.runner().run(&["kill-window", "-t", &target])
    }
}
