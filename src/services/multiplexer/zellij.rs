//! Zellij backend implementation.

use std::fs;
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

    /// Create a session with a layout file that includes the command
    fn create_session_with_layout(
        &self,
        session: &str,
        window: &str,
        cwd: &str,
        command: &str,
    ) -> Result<()> {
        // Escape command for KDL format (escape backslashes and quotes)
        let escaped_cmd = command.replace('\\', "\\\\").replace('"', "\\\"");

        // Create layout content
        let layout = format!(
            r#"layout {{
    tab name="{window}" cwd="{cwd}" {{
        pane command="bash" {{
            args "-c" "{escaped_cmd}"
        }}
    }}
}}"#
        );

        // Write to temp file
        let layout_path = format!("/tmp/wt-zellij-{}.kdl", std::process::id());
        fs::write(&layout_path, &layout).map_err(|e| {
            WtError::Zellij(format!("Failed to write layout file: {}", e))
        })?;

        // Create session with layout
        let _ = Command::new("zellij")
            .args(["--session", session, "--new-session-with-layout", &layout_path])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        std::thread::sleep(Duration::from_millis(500));

        // Clean up layout file
        let _ = fs::remove_file(&layout_path);

        if self.session_exists(session) {
            Ok(())
        } else {
            Err(WtError::Zellij(format!(
                "Failed to create session '{}' with layout",
                session
            )))
        }
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
        // Use attach -b -c to create a detached session
        let _ = Command::new("zellij")
            .args(["attach", session, "-b", "-c"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        std::thread::sleep(Duration::from_millis(300));

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
        // Strategy: Always use layout file to create session with tab and command.
        // This avoids the blocking issue with go-to-tab-name and write-chars.
        //
        // If session exists, delete it first (wt typically uses one session per task).

        if self.session_exists(session) {
            // Delete existing session to start fresh
            let _ = Command::new("zellij")
                .args(["delete-session", session, "--force"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            std::thread::sleep(Duration::from_millis(200));
        }

        if !command.is_empty() {
            // Create session with layout that includes the command
            self.create_session_with_layout(session, window, cwd, command)
        } else {
            // No command - create simple session with tab
            self.create_session_with_layout(session, window, cwd, "exec $SHELL")
        }
    }

    fn window_exists(&self, session: &str, window: &str) -> bool {
        // zellij -s <session> action query-tab-names | grep -q <window>
        self.runner()
            .output(&["-s", session, "action", "query-tab-names"])
            .map(|out| out.lines().any(|line| line.trim() == window))
            .unwrap_or(false)
    }

    fn kill_window(&self, session: &str, _window: &str) -> Result<()> {
        // Note: zellij's go-to-tab-name blocks without a tty, so we can't switch to
        // a specific tab to close it. Instead, we delete the entire session.
        // This works because wt typically uses one session per task.
        //
        // zellij delete-session <session> --force
        Command::new("zellij")
            .args(["delete-session", session, "--force"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .ok();

        Ok(())
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
