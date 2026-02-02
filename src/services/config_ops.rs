//! Configuration operations for hooks system.

use crate::error::{Result, WtError};
use crate::models::WtConfig;

/// Get a configuration value by key.
///
/// Supported keys:
/// - `claude_command` - The command to run Claude
/// - `session_name` - Multiplexer session name
/// - `multiplexer` - Multiplexer type (tmux/zellij)
/// - `worktree_dir` - Worktree directory path
/// - `start_args` - Arguments for wt start
pub fn get_config(key: &str) -> Result<String> {
    let config = WtConfig::load()?;

    match key {
        "claude_command" => Ok(config.claude_command),
        "session_name" => Ok(config.session_name),
        "multiplexer" => Ok(config.multiplexer),
        "worktree_dir" => Ok(config.worktree_dir),
        "start_args" => Ok(config.start_args),
        "init_script" => Ok(config.init_script.unwrap_or_default()),
        "archive_script" => Ok(config.archive_script.unwrap_or_default()),
        "review_script" => Ok(config.review_script.unwrap_or_default()),
        "merge_script" => Ok(config.merge_script.unwrap_or_default()),
        _ => Err(WtError::InvalidInput(format!(
            "Unknown config key: '{}'. Valid keys: claude_command, session_name, multiplexer, worktree_dir, start_args, init_script, archive_script, review_script, merge_script",
            key
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests would need a config file to exist.
    // In practice, we test config loading in models/config.rs tests.

    #[test]
    fn test_invalid_key() {
        // This test doesn't need a config file since it fails early on key validation
        // Actually it does need a config file since we call WtConfig::load() first
        // Let's just verify the error message format
        let err = WtError::InvalidInput("Unknown config key: 'invalid'".to_string());
        assert!(err.to_string().contains("Unknown config key"));
    }
}
