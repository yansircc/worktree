//! Terminal multiplexer abstraction layer.
//!
//! This module provides a unified interface for different terminal multiplexers
//! (tmux, zellij, etc.) to manage sessions and windows/tabs for parallel
//! agent development.

mod tmux;
mod zellij;

pub use tmux::TmuxBackend;
pub use zellij::ZellijBackend;

use crate::error::{Result, WtError};
use serde::{Deserialize, Serialize};

/// Supported multiplexer backends
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MultiplexerType {
    #[default]
    Tmux,
    Zellij,
}

impl MultiplexerType {
    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "tmux" => Some(Self::Tmux),
            "zellij" => Some(Self::Zellij),
            _ => None,
        }
    }

    /// Get the binary name for this multiplexer
    pub fn binary_name(&self) -> &'static str {
        match self {
            Self::Tmux => "tmux",
            Self::Zellij => "zellij",
        }
    }
}

impl std::fmt::Display for MultiplexerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tmux => write!(f, "tmux"),
            Self::Zellij => write!(f, "zellij"),
        }
    }
}

/// A terminal multiplexer backend for managing sessions and windows/tabs.
///
/// This trait abstracts the operations needed by wt to manage parallel
/// agent processes across different multiplexer implementations.
pub trait Multiplexer: Send + Sync {
    /// Check if the multiplexer binary is installed and available
    fn is_available(&self) -> bool;

    /// Check if a session exists
    fn session_exists(&self, session: &str) -> bool;

    /// Create a new session (detached)
    fn create_session(&self, session: &str) -> Result<()>;

    /// Create a new window/tab in a session with a command
    ///
    /// # Arguments
    /// * `session` - Session name
    /// * `window` - Window/tab name
    /// * `cwd` - Working directory
    /// * `command` - Command to execute (can be empty for shell)
    fn create_window(&self, session: &str, window: &str, cwd: &str, command: &str) -> Result<()>;

    /// Check if a window/tab exists in a session
    fn window_exists(&self, session: &str, window: &str) -> bool;

    /// Kill (close) a window/tab
    fn kill_window(&self, session: &str, window: &str) -> Result<()>;

    /// Kill a window/tab if it exists, returning whether it was killed
    fn kill_window_if_exists(&self, session: &str, window: &str) -> Result<bool> {
        if self.window_exists(session, window) {
            self.kill_window(session, window)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Focus (select) a window/tab
    fn focus_window(&self, session: &str, window: &str) -> Result<()>;

    /// Send keys to a window/tab
    fn send_keys(&self, session: &str, window: &str, keys: &str) -> Result<()>;

    /// List all windows/tabs in a session
    fn list_windows(&self, session: &str) -> Result<Vec<String>>;
}

/// Factory function to create a multiplexer backend
pub fn create_multiplexer(mux_type: MultiplexerType) -> Box<dyn Multiplexer> {
    match mux_type {
        MultiplexerType::Tmux => Box::new(TmuxBackend::new()),
        MultiplexerType::Zellij => Box::new(ZellijBackend::new()),
    }
}

/// Check if a multiplexer is installed and return error if not
pub fn check_multiplexer_installed(mux_type: MultiplexerType) -> Result<()> {
    let mux = create_multiplexer(mux_type);
    if !mux.is_available() {
        return Err(WtError::MultiplexerNotInstalled(
            mux_type.binary_name().to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== MultiplexerType::from_str Tests ====================

    #[test]
    fn test_multiplexer_type_from_str() {
        assert_eq!(
            MultiplexerType::from_str("tmux"),
            Some(MultiplexerType::Tmux)
        );
        assert_eq!(
            MultiplexerType::from_str("TMUX"),
            Some(MultiplexerType::Tmux)
        );
        assert_eq!(
            MultiplexerType::from_str("zellij"),
            Some(MultiplexerType::Zellij)
        );
        assert_eq!(
            MultiplexerType::from_str("ZELLIJ"),
            Some(MultiplexerType::Zellij)
        );
        assert_eq!(MultiplexerType::from_str("unknown"), None);
    }

    #[test]
    fn test_multiplexer_type_from_str_mixed_case() {
        assert_eq!(
            MultiplexerType::from_str("Tmux"),
            Some(MultiplexerType::Tmux)
        );
        assert_eq!(
            MultiplexerType::from_str("Zellij"),
            Some(MultiplexerType::Zellij)
        );
        assert_eq!(
            MultiplexerType::from_str("TmUx"),
            Some(MultiplexerType::Tmux)
        );
    }

    #[test]
    fn test_multiplexer_type_from_str_invalid() {
        assert_eq!(MultiplexerType::from_str(""), None);
        assert_eq!(MultiplexerType::from_str(" "), None);
        assert_eq!(MultiplexerType::from_str("screen"), None);
        assert_eq!(MultiplexerType::from_str("tmux "), None); // trailing space
    }

    // ==================== MultiplexerType::display Tests ====================

    #[test]
    fn test_multiplexer_type_display() {
        assert_eq!(format!("{}", MultiplexerType::Tmux), "tmux");
        assert_eq!(format!("{}", MultiplexerType::Zellij), "zellij");
    }

    // ==================== MultiplexerType::binary_name Tests ====================

    #[test]
    fn test_multiplexer_type_binary_name() {
        assert_eq!(MultiplexerType::Tmux.binary_name(), "tmux");
        assert_eq!(MultiplexerType::Zellij.binary_name(), "zellij");
    }

    // ==================== MultiplexerType::default Tests ====================

    #[test]
    fn test_multiplexer_type_default() {
        assert_eq!(MultiplexerType::default(), MultiplexerType::Tmux);
    }

    // ==================== create_multiplexer Tests ====================

    #[test]
    fn test_create_multiplexer_tmux() {
        let mux = create_multiplexer(MultiplexerType::Tmux);
        // Can't directly test the type, but we can verify it doesn't panic
        // and returns a valid trait object
        let _ = mux.is_available();
    }

    #[test]
    fn test_create_multiplexer_zellij() {
        let mux = create_multiplexer(MultiplexerType::Zellij);
        let _ = mux.is_available();
    }

    // ==================== MultiplexerType Serialization Tests ====================

    #[test]
    fn test_multiplexer_type_serialize() {
        let tmux = MultiplexerType::Tmux;
        let json = serde_json::to_string(&tmux).unwrap();
        assert_eq!(json, "\"tmux\"");

        let zellij = MultiplexerType::Zellij;
        let json = serde_json::to_string(&zellij).unwrap();
        assert_eq!(json, "\"zellij\"");
    }

    #[test]
    fn test_multiplexer_type_deserialize() {
        let tmux: MultiplexerType = serde_json::from_str("\"tmux\"").unwrap();
        assert_eq!(tmux, MultiplexerType::Tmux);

        let zellij: MultiplexerType = serde_json::from_str("\"zellij\"").unwrap();
        assert_eq!(zellij, MultiplexerType::Zellij);
    }

    #[test]
    fn test_multiplexer_type_clone_copy() {
        let original = MultiplexerType::Tmux;
        let cloned = original.clone();
        let copied = original; // Copy trait
        assert_eq!(original, cloned);
        assert_eq!(original, copied);
    }
}
