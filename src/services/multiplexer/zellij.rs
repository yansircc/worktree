//! Zellij backend implementation.

use std::process::{Command, Stdio};
use std::time::Duration;

use crate::error::{Result, WtError};
use crate::services::command::CommandRunner;

use super::{Multiplexer, MultiplexerType};

/// Zellij multiplexer backend
pub struct ZellijBackend;

impl ZellijBackend {
    pub fn new() -> Self {
        Self
    }

    fn runner(&self) -> CommandRunner {
        CommandRunner::zellij()
    }
}

impl Default for ZellijBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Multiplexer for ZellijBackend {
    fn multiplexer_type(&self) -> MultiplexerType {
        MultiplexerType::Zellij
    }

    fn is_available(&self) -> bool {
        Command::new("zellij")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn session_exists(&self, session: &str) -> bool {
        // zellij list-sessions -s | grep -q <session>
        self.runner()
            .output(&["list-sessions", "-s"])
            .map(|out| out.lines().any(|line| line.trim() == session))
            .unwrap_or(false)
    }

    fn create_session(&self, session: &str) -> Result<()> {
        // zellij -s <session> --new-session-with-layout compact
        // Note: This may panic with "not a tty" but session is still created
        // We spawn detached and ignore the error
        let _ = Command::new("zellij")
            .args(["-s", session, "--new-session-with-layout", "compact"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        // Give zellij a moment to create the session
        std::thread::sleep(Duration::from_millis(500));

        // Verify session was created
        if self.session_exists(session) {
            Ok(())
        } else {
            Err(WtError::Zellij(format!(
                "Failed to create session '{}'",
                session
            )))
        }
    }

    fn create_window(&self, session: &str, window: &str, cwd: &str, command: &str) -> Result<()> {
        // zellij -s <session> action new-tab -n <window> -c <cwd>
        self.runner()
            .run(&["-s", session, "action", "new-tab", "-n", window, "-c", cwd])?;

        // Send command if provided
        if !command.is_empty() {
            // zellij -s <session> action write-chars <command>
            self.runner()
                .run(&["-s", session, "action", "write-chars", command])?;

            // Send Enter (byte 10 = newline)
            self.runner()
                .run(&["-s", session, "action", "write", "10"])?;
        }

        Ok(())
    }

    fn window_exists(&self, session: &str, window: &str) -> bool {
        // zellij -s <session> action query-tab-names | grep -q <window>
        self.runner()
            .output(&["-s", session, "action", "query-tab-names"])
            .map(|out| out.lines().any(|line| line.trim() == window))
            .unwrap_or(false)
    }

    fn kill_window(&self, session: &str, window: &str) -> Result<()> {
        // First go to the tab, then close it
        // zellij -s <session> action go-to-tab-name <window>
        // zellij -s <session> action close-tab

        // Go to tab (ignore error if tab doesn't exist)
        let _ = self
            .runner()
            .run(&["-s", session, "action", "go-to-tab-name", window]);

        // Close current tab
        self.runner()
            .run(&["-s", session, "action", "close-tab"])
    }

    fn kill_window_if_exists(&self, session: &str, window: &str) -> Result<bool> {
        if self.window_exists(session, window) {
            // Silently ignore errors when closing (tab might have closed between check and close)
            let _ = self.kill_window(session, window);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zellij_backend_type() {
        let backend = ZellijBackend::new();
        assert_eq!(backend.multiplexer_type(), MultiplexerType::Zellij);
    }
}
