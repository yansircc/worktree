//! Zellij backend implementation.
//!
//! Note: Due to zellij limitations (blocking commands without tty), this backend
//! uses a simplified model where each task gets its own session. The `kill_window`
//! method actually deletes the entire session.

use std::fs;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::error::{Result, WtError};
use crate::services::command::CommandRunner;

use super::Multiplexer;

/// Time to wait for session creation with layout
const SESSION_CREATE_WAIT_MS: u64 = 500;
/// Time to wait for simple session creation
const SESSION_ATTACH_WAIT_MS: u64 = 300;
/// Time to wait for session deletion
const SESSION_DELETE_WAIT_MS: u64 = 200;

/// Zellij multiplexer backend
///
/// Due to zellij's design, some operations that work well with tmux are problematic:
/// - `go-to-tab-name` and `write-chars` block without a tty
/// - Cannot easily create a new tab in an existing session from a script
///
/// Therefore, this backend uses a "one session per task" model:
/// - `create_window` creates a new session with the tab and command via layout file
/// - `kill_window` deletes the entire session (not just a tab)
pub struct ZellijBackend;

impl ZellijBackend {
    pub fn new() -> Self {
        Self
    }

    fn runner(&self) -> CommandRunner {
        CommandRunner::zellij()
    }

    /// Generate a unique layout file path
    fn layout_path() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("/tmp/wt-zellij-{}-{}.kdl", std::process::id(), timestamp)
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

        // Write to temp file with unique name
        let layout_path = Self::layout_path();
        fs::write(&layout_path, &layout)
            .map_err(|e| WtError::Zellij(format!("Failed to write layout file: {}", e)))?;

        // Create session with layout
        let spawn_result = Command::new("zellij")
            .args([
                "--session",
                session,
                "--new-session-with-layout",
                &layout_path,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        if let Err(e) = spawn_result {
            let _ = fs::remove_file(&layout_path);
            return Err(WtError::Zellij(format!("Failed to spawn zellij: {}", e)));
        }

        std::thread::sleep(Duration::from_millis(SESSION_CREATE_WAIT_MS));

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
        self.runner()
            .output(&["list-sessions", "-s"])
            .map(|out| out.lines().any(|line| line.trim() == session))
            .unwrap_or(false)
    }

    fn create_session(&self, session: &str) -> Result<()> {
        let status = Command::new("zellij")
            .args(["attach", session, "-b", "-c"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        if let Err(e) = status {
            return Err(WtError::Zellij(format!("Failed to run zellij: {}", e)));
        }

        std::thread::sleep(Duration::from_millis(SESSION_ATTACH_WAIT_MS));

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
        // If session exists, delete it first (wt uses one session per task).

        if self.session_exists(session) {
            let _ = Command::new("zellij")
                .args(["delete-session", session, "--force"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            std::thread::sleep(Duration::from_millis(SESSION_DELETE_WAIT_MS));
        }

        let cmd = if command.is_empty() {
            "exec $SHELL"
        } else {
            command
        };
        self.create_session_with_layout(session, window, cwd, cmd)
    }

    fn window_exists(&self, session: &str, window: &str) -> bool {
        self.runner()
            .output(&["-s", session, "action", "query-tab-names"])
            .map(|out| out.lines().any(|line| line.trim() == window))
            .unwrap_or(false)
    }

    /// Kill a window (actually deletes the entire session).
    ///
    /// Note: Due to zellij limitations, we cannot close a specific tab without a tty.
    /// Since wt uses one session per task, deleting the session is the expected behavior.
    fn kill_window(&self, session: &str, _window: &str) -> Result<()> {
        let _ = Command::new("zellij")
            .args(["delete-session", session, "--force"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        Ok(())
    }

    fn kill_window_if_exists(&self, session: &str, window: &str) -> Result<bool> {
        if self.window_exists(session, window) {
            let _ = self.kill_window(session, window);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
