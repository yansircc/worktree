//! Zellij backend implementation.
//!
//! Uses a two-step process for creating windows to support parallel task creation:
//! 1. `zellij attach -b -c` creates a detached session in background
//! 2. `zellij action new-tab --layout` adds tab without attaching
//!
//! This avoids the blocking behavior of `--new-session-with-layout` which attaches
//! to the session and prevents parallel task startup.

use std::fs;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::error::{Result, WtError};
use crate::services::command::CommandRunner;

use super::Multiplexer;

/// Time to wait for tab creation with layout
const SESSION_CREATE_WAIT_MS: u64 = 500;
/// Time to wait for session creation
const SESSION_ATTACH_WAIT_MS: u64 = 300;

/// Zellij multiplexer backend
///
/// Supports parallel task creation by using non-blocking operations:
/// - `create_window` creates session in background, then adds tab via action
/// - Multiple tasks can be created concurrently without blocking
///
/// Note: Some zellij operations still require a tty (`go-to-tab-name`, `write-chars`),
/// so `focus_window` and `send_keys` return errors.
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

    /// Create a session in background and add a tab with command via layout.
    ///
    /// Uses two-step process to avoid blocking:
    /// 1. `zellij attach -b -c` creates detached session in background
    /// 2. `zellij action new-tab --layout` adds tab without attaching
    fn create_session_with_layout(
        &self,
        session: &str,
        window: &str,
        cwd: &str,
        command: &str,
    ) -> Result<()> {
        // Step 1: Create detached session in background if it doesn't exist
        if !self.session_exists(session) {
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

            if !self.session_exists(session) {
                return Err(WtError::Zellij(format!(
                    "Failed to create session '{}'",
                    session
                )));
            }
        }

        // Step 2: Add tab with command via layout file
        // Escape command for KDL format (escape backslashes and quotes)
        let escaped_cmd = command.replace('\\', "\\\\").replace('"', "\\\"");

        // Create layout content (for action new-tab, we still use full layout format)
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

        // Add tab to session via action (non-blocking, doesn't attach)
        let status = Command::new("zellij")
            .args(["-s", session, "action", "new-tab", "--layout", &layout_path])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        // Clean up layout file
        let _ = fs::remove_file(&layout_path);

        if let Err(e) = status {
            return Err(WtError::Zellij(format!(
                "Failed to create tab '{}': {}",
                window, e
            )));
        }

        std::thread::sleep(Duration::from_millis(SESSION_CREATE_WAIT_MS));

        Ok(())
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
        // Strategy: Create session in background if needed, then add tab via action.
        // This is fully non-blocking and supports parallel task creation.

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

    fn focus_window(&self, session: &str, window: &str) -> Result<()> {
        // zellij's go-to-tab-name action requires a tty, which we don't have
        // in a script context. Return an error explaining the limitation.
        Err(WtError::Zellij(format!(
            "Cannot focus window '{}' in session '{}': zellij requires an interactive terminal for this operation",
            window, session
        )))
    }

    fn send_keys(&self, session: &str, window: &str, keys: &str) -> Result<()> {
        // zellij's write-chars action requires a tty, which we don't have
        // in a script context. Return an error explaining the limitation.
        Err(WtError::Zellij(format!(
            "Cannot send keys '{}' to window '{}' in session '{}': zellij requires an interactive terminal for this operation",
            keys, window, session
        )))
    }

    fn list_windows(&self, session: &str) -> Result<Vec<String>> {
        self.runner()
            .output(&["-s", session, "action", "query-tab-names"])
            .map(|out| out.lines().map(|s| s.trim().to_string()).collect())
    }
}
