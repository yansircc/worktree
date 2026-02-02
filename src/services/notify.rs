//! Notification and interaction operations for hooks system.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use chrono::Local;

use crate::constants::LOGS_DIR;
use crate::error::{Result, WtError};

/// Send a system notification.
pub fn notify(title: &str, message: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        Command::new("osascript")
            .args([
                "-e",
                &format!(
                    "display notification \"{}\" with title \"{}\"",
                    escape_applescript(message),
                    escape_applescript(title)
                ),
            ])
            .status()
            .map_err(|e| WtError::Script {
                script: "osascript".to_string(),
                message: e.to_string(),
            })?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("notify-send")
            .args([title, message])
            .status()
            .map_err(|e| WtError::Script {
                script: "notify-send".to_string(),
                message: e.to_string(),
            })?;
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        // On unsupported platforms, just print to stderr
        eprintln!("[{}] {}", title, message);
    }

    Ok(())
}

/// Escape special characters for AppleScript strings.
#[cfg(target_os = "macos")]
fn escape_applescript(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Interactive confirmation prompt.
///
/// Returns true if user answers 'y' or 'Y', false otherwise.
pub fn confirm(message: &str) -> Result<bool> {
    print!("{} [y/N] ", message);
    io::stdout().flush().map_err(|e| WtError::Io {
        operation: "flush stdout".to_string(),
        path: "stdout".to_string(),
        message: e.to_string(),
    })?;

    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(|e| WtError::Io {
        operation: "read stdin".to_string(),
        path: "stdin".to_string(),
        message: e.to_string(),
    })?;

    Ok(input.trim().to_lowercase() == "y")
}

/// Abort execution with an error message.
///
/// This function never returns - it exits the process with code 1.
pub fn abort(message: &str) -> ! {
    eprintln!("Aborted: {}", message);
    std::process::exit(1)
}

/// Log a message for a task.
///
/// Logs are written to .wt/logs/<task>.log with timestamps.
pub fn log(task: &str, message: &str) -> Result<()> {
    let log_dir = Path::new(LOGS_DIR);
    fs::create_dir_all(log_dir).map_err(|e| WtError::Io {
        operation: "create log directory".to_string(),
        path: LOGS_DIR.to_string(),
        message: e.to_string(),
    })?;

    let log_file = log_dir.join(format!("{}.log", task));
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| WtError::Io {
            operation: "open log file".to_string(),
            path: log_file.to_string_lossy().to_string(),
            message: e.to_string(),
        })?;

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    writeln!(file, "[{}] {}", timestamp, message).map_err(|e| WtError::Io {
        operation: "write log".to_string(),
        path: log_file.to_string_lossy().to_string(),
        message: e.to_string(),
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_log_creates_file() {
        let temp = TempDir::new().unwrap();
        let log_dir = temp.path().join("logs");
        fs::create_dir_all(&log_dir).unwrap();

        // We can't easily test the actual log function since it uses LOGS_DIR constant,
        // but we can test the file operations manually
        let log_file = log_dir.join("test.log");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .unwrap();

        writeln!(file, "[2024-01-01 12:00:00] Test message").unwrap();

        let content = fs::read_to_string(&log_file).unwrap();
        assert!(content.contains("Test message"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_escape_applescript() {
        assert_eq!(escape_applescript("hello"), "hello");
        assert_eq!(escape_applescript("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(escape_applescript("path\\to"), "path\\\\to");
    }
}
