//! Transcript service for reading Claude Code session transcripts.

use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use crate::models::Instance;

/// Metrics extracted from transcript
#[derive(Debug, Default, Clone)]
pub struct TranscriptMetrics {
    /// Input tokens (context size from last assistant message)
    pub input_tokens: u64,
    /// Output tokens (cumulative)
    pub output_tokens: u64,
    /// Number of conversation turns
    pub num_turns: u32,
    /// Context window size
    pub context_window: u64,
    /// Final summary/result from last assistant message
    pub summary: Option<String>,
    /// Whether the session completed normally
    pub completed: bool,
    /// Session start timestamp (from first entry)
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Session end timestamp (from last entry)
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Current tool being used (last tool_use)
    pub current_tool: Option<String>,
}

impl TranscriptMetrics {
    /// Calculate context usage percentage
    pub fn context_percent(&self) -> u8 {
        if self.context_window == 0 {
            return 0;
        }
        let used = self.input_tokens + self.output_tokens;
        ((used * 100) / self.context_window).min(100) as u8
    }

    /// Calculate session duration in seconds
    pub fn duration_secs(&self) -> Option<i64> {
        match (self.started_at, self.finished_at) {
            (Some(start), Some(end)) => Some(end.signed_duration_since(start).num_seconds()),
            _ => None,
        }
    }
}

/// Convert a filesystem path to Claude Code's project directory name.
///
/// Claude Code escapes paths by replacing `/` and `.` with `-`.
/// Example: `/Users/foo/project/.wt` -> `-Users-foo-project--wt`
pub fn project_dir_name(path: &str) -> String {
    path.replace(['/', '.'], "-")
}

/// Get the Claude Code projects directory.
pub fn claude_projects_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude/projects"))
}

/// Get the transcript file path for a given worktree path and session ID.
pub fn transcript_path(worktree_path: &str, session_id: &str) -> Option<PathBuf> {
    let projects_dir = claude_projects_dir()?;
    let dir_name = project_dir_name(worktree_path);
    Some(
        projects_dir
            .join(dir_name)
            .join(format!("{}.jsonl", session_id)),
    )
}

/// 查找 Instance 对应的 transcript 文件
/// 优先使用 session_id 精确匹配，否则查找最新的 transcript
pub fn find_transcript_for_instance(instance: &Instance) -> Option<PathBuf> {
    let worktree_path = instance.worktree_path.as_deref()?;
    instance
        .session_id
        .as_ref()
        .and_then(|sid| transcript_path(worktree_path, sid))
        .filter(|p: &PathBuf| p.exists())
        .or_else(|| find_latest_transcript(worktree_path))
}

/// Find the most recent transcript file for a worktree.
/// This is more reliable than using our generated session_id since Claude
/// generates its own session IDs.
pub fn find_latest_transcript(worktree_path: &str) -> Option<PathBuf> {
    let projects_dir = claude_projects_dir()?;
    let dir_name = project_dir_name(worktree_path);
    let project_dir = projects_dir.join(dir_name);

    if !project_dir.exists() {
        return None;
    }

    // Find all .jsonl files and get the most recently modified one
    std::fs::read_dir(&project_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext == "jsonl")
                .unwrap_or(false)
        })
        .max_by_key(|entry| entry.metadata().ok().and_then(|m| m.modified().ok()))
        .map(|entry| entry.path())
}

/// Parse a transcript file and extract metrics.
pub fn parse_transcript(path: &Path) -> Option<TranscriptMetrics> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut metrics = TranscriptMetrics::default();
    metrics.context_window = 200_000; // Default

    let mut last_cache_read: u64 = 0;
    let mut last_input: u64 = 0;
    let mut total_output: u64 = 0;
    let mut turn_count: u32 = 0;
    let mut last_assistant_text: Option<String> = None;
    let mut first_timestamp: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut last_timestamp: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut last_tool: Option<String> = None;

    for line in reader.lines() {
        let line = line.ok()?;
        if line.is_empty() {
            continue;
        }

        if let Ok(entry) = serde_json::from_str::<TranscriptEntry>(&line) {
            // Extract timestamp from every entry
            if let Some(ts) = &entry.timestamp {
                if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(ts) {
                    let utc = parsed.with_timezone(&chrono::Utc);
                    if first_timestamp.is_none() {
                        first_timestamp = Some(utc);
                    }
                    last_timestamp = Some(utc);
                }
            }

            match entry.r#type.as_str() {
                "assistant" => {
                    if let Some(msg) = entry.message {
                        // Extract usage info
                        if let Some(usage) = msg.usage {
                            last_cache_read = usage.cache_read_input_tokens.unwrap_or(0);
                            last_input = usage.input_tokens.unwrap_or(0);
                            total_output += usage.output_tokens.unwrap_or(0);
                        }

                        // Extract text content for summary and tool usage
                        if let Some(content) = msg.content {
                            for item in content {
                                if item.r#type == "text" {
                                    if let Some(text) = item.text {
                                        last_assistant_text = Some(text);
                                    }
                                } else if item.r#type == "tool_use" {
                                    if let Some(name) = &item.name {
                                        last_tool = Some(name.clone());
                                    }
                                }
                            }
                        }

                        turn_count += 1;
                    }
                }
                "system" => {
                    // Check for init entry to get context window
                    if entry.subtype.as_deref() == Some("init") {
                        // Could extract model info here if needed
                    }
                }
                _ => {}
            }
        }
    }

    // Context = cache_read (history) + input (new tokens)
    metrics.input_tokens = last_cache_read + last_input;
    metrics.output_tokens = total_output;
    metrics.num_turns = turn_count;
    metrics.summary = last_assistant_text;
    metrics.completed = turn_count > 0; // Consider completed if there's at least one turn
    metrics.started_at = first_timestamp;
    metrics.finished_at = last_timestamp;
    metrics.current_tool = last_tool;

    Some(metrics)
}

// Deserialization structs

#[derive(Debug, Deserialize)]
struct TranscriptEntry {
    r#type: String,
    #[serde(default)]
    subtype: Option<String>,
    #[serde(default)]
    message: Option<TranscriptMessage>,
    #[serde(default)]
    timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TranscriptMessage {
    #[serde(default)]
    usage: Option<Usage>,
    #[serde(default)]
    content: Option<Vec<ContentItem>>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct ContentItem {
    r#type: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    thinking: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

/// Get the last N assistant messages from a transcript.
/// Extracts text content first, falls back to thinking content if no text.
pub fn get_last_messages(path: &Path, n: usize) -> Option<Vec<String>> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut messages: Vec<String> = Vec::new();

    for line in reader.lines() {
        let line = line.ok()?;
        if line.is_empty() {
            continue;
        }

        if let Ok(entry) = serde_json::from_str::<TranscriptEntry>(&line) {
            if entry.r#type == "assistant" {
                if let Some(msg) = entry.message {
                    if let Some(content) = msg.content {
                        // Try to get text content first
                        let text_parts: Vec<String> = content
                            .iter()
                            .filter(|item| item.r#type == "text")
                            .filter_map(|item| item.text.clone())
                            .collect();

                        if !text_parts.is_empty() {
                            messages.push(text_parts.join("\n"));
                        } else {
                            // Fall back to thinking content
                            let thinking_parts: Vec<String> = content
                                .iter()
                                .filter(|item| item.r#type == "thinking")
                                .filter_map(|item| item.thinking.clone())
                                .collect();

                            if !thinking_parts.is_empty() {
                                messages.push(thinking_parts.join("\n"));
                            }
                        }
                    }
                }
            }
        }
    }

    // Return last N messages
    let start = messages.len().saturating_sub(n);
    Some(messages[start..].to_vec())
}

/// Extract filtered transcript to a log file.
/// Returns the number of entries written.
pub fn extract_to_log(
    transcript_path: &Path,
    log_path: &Path,
    exclude_types: &[String],
    exclude_fields: &[String],
) -> Option<usize> {
    let file = File::open(transcript_path).ok()?;
    let reader = BufReader::new(file);

    // Ensure parent directory exists
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent).ok()?;
    }

    let mut output = File::create(log_path).ok()?;
    let mut count = 0;

    for line in reader.lines() {
        let line = line.ok()?;
        if line.is_empty() {
            continue;
        }

        // Parse as generic JSON
        let mut json: Value = serde_json::from_str(&line).ok()?;

        // Check if type should be excluded
        if let Some(entry_type) = json.get("type").and_then(|v| v.as_str()) {
            if exclude_types.iter().any(|t| t == entry_type) {
                continue;
            }
        }

        // Remove excluded fields recursively
        remove_fields(&mut json, exclude_fields);

        // Write filtered entry
        writeln!(output, "{}", serde_json::to_string(&json).ok()?).ok()?;
        count += 1;
    }

    Some(count)
}

/// Recursively remove specified fields from a JSON value.
fn remove_fields(value: &mut Value, fields: &[String]) {
    match value {
        Value::Object(map) => {
            // Remove specified fields
            for field in fields {
                map.remove(field);
            }
            // Recurse into remaining values
            for v in map.values_mut() {
                remove_fields(v, fields);
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                remove_fields(v, fields);
            }
        }
        _ => {}
    }
}

/// Generate log file path for a task.
/// Structure: .wt/logs/<task>/<session_id_prefix>.jsonl
pub fn log_path(task_name: &str, session_id: &str) -> PathBuf {
    let short_session = &session_id[..8.min(session_id.len())];
    PathBuf::from(crate::constants::LOGS_DIR)
        .join(task_name)
        .join(format!("{}.jsonl", short_session))
}

/// Get the latest assistant message from a transcript, truncated for TUI display.
/// Returns a short summary like "> Editing src/main.rs..." or "> I'll implement..."
pub fn get_latest_message(path: &Path, max_len: usize) -> Option<String> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut latest_text: Option<String> = None;
    let mut latest_tool: Option<String> = None;

    for line in reader.lines() {
        let line = line.ok()?;
        if line.is_empty() {
            continue;
        }

        if let Ok(entry) = serde_json::from_str::<TranscriptEntry>(&line) {
            if entry.r#type == "assistant" {
                if let Some(msg) = entry.message {
                    if let Some(content) = msg.content {
                        for item in content {
                            if item.r#type == "text" {
                                if let Some(text) = item.text {
                                    // Get first non-empty line
                                    let first_line = text
                                        .lines()
                                        .find(|l| !l.trim().is_empty())
                                        .unwrap_or(&text);
                                    latest_text = Some(first_line.to_string());
                                }
                            } else if item.r#type == "tool_use" {
                                if let Some(name) = &item.name {
                                    latest_tool = Some(name.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Prefer tool use description, fall back to text
    let message = if let Some(tool) = latest_tool {
        format_tool_message(&tool)
    } else if let Some(text) = latest_text {
        text
    } else {
        return None;
    };

    // Truncate to max_len
    Some(truncate_message(&message, max_len))
}

/// Format a tool use into a human-readable message
fn format_tool_message(tool: &str) -> String {
    match tool {
        "Read" => "Reading file...".to_string(),
        "Write" => "Writing file...".to_string(),
        "Edit" => "Editing file...".to_string(),
        "Bash" => "Running command...".to_string(),
        "Glob" => "Searching files...".to_string(),
        "Grep" => "Searching content...".to_string(),
        "Task" => "Running sub-agent...".to_string(),
        "WebFetch" => "Fetching web content...".to_string(),
        "WebSearch" => "Searching web...".to_string(),
        other => {
            if other.starts_with("mcp__") {
                let short = other.rsplit("__").next().unwrap_or(other);
                format!("Using {}...", short)
            } else {
                format!("Using {}...", other)
            }
        }
    }
}

/// Truncate message to max length, adding ellipsis if needed
fn truncate_message(s: &str, max_len: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated.trim_end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== project_dir_name Tests ====================

    #[test]
    fn test_project_dir_name() {
        assert_eq!(project_dir_name("/Users/foo/project"), "-Users-foo-project");
        // . is also replaced with -
        assert_eq!(
            project_dir_name("/Users/foo/project/.wt-worktrees/task"),
            "-Users-foo-project--wt-worktrees-task"
        );
    }

    #[test]
    fn test_project_dir_name_edge_cases() {
        assert_eq!(project_dir_name(""), "");
        assert_eq!(project_dir_name("."), "-");
        assert_eq!(project_dir_name("/"), "-");
        assert_eq!(project_dir_name(".."), "--");
        assert_eq!(project_dir_name("./foo"), "--foo");
    }

    // ==================== TranscriptMetrics::context_percent Tests ====================

    #[test]
    fn test_context_percent() {
        let metrics = TranscriptMetrics {
            input_tokens: 50_000,
            output_tokens: 10_000,
            context_window: 200_000,
            ..Default::default()
        };
        assert_eq!(metrics.context_percent(), 30);
    }

    #[test]
    fn test_context_percent_zero_window() {
        let metrics = TranscriptMetrics {
            input_tokens: 50_000,
            output_tokens: 10_000,
            context_window: 0,
            ..Default::default()
        };
        assert_eq!(metrics.context_percent(), 0);
    }

    #[test]
    fn test_context_percent_over_100() {
        // Should cap at 100%
        let metrics = TranscriptMetrics {
            input_tokens: 150_000,
            output_tokens: 100_000,
            context_window: 200_000,
            ..Default::default()
        };
        assert_eq!(metrics.context_percent(), 100);
    }

    #[test]
    fn test_context_percent_exact_100() {
        let metrics = TranscriptMetrics {
            input_tokens: 100_000,
            output_tokens: 100_000,
            context_window: 200_000,
            ..Default::default()
        };
        assert_eq!(metrics.context_percent(), 100);
    }

    // ==================== TranscriptMetrics::duration_secs Tests ====================

    #[test]
    fn test_duration_secs_none() {
        let metrics = TranscriptMetrics::default();
        assert!(metrics.duration_secs().is_none());
    }

    #[test]
    fn test_duration_secs_only_start() {
        let metrics = TranscriptMetrics {
            started_at: Some(chrono::Utc::now()),
            ..Default::default()
        };
        assert!(metrics.duration_secs().is_none());
    }

    #[test]
    fn test_duration_secs_valid() {
        use chrono::{Duration, Utc};
        let start = Utc::now();
        let end = start + Duration::seconds(120);
        let metrics = TranscriptMetrics {
            started_at: Some(start),
            finished_at: Some(end),
            ..Default::default()
        };
        assert_eq!(metrics.duration_secs(), Some(120));
    }

    // ==================== truncate_message Tests ====================

    #[test]
    fn test_truncate_message_short() {
        assert_eq!(truncate_message("Hello", 10), "Hello");
    }

    #[test]
    fn test_truncate_message_exact() {
        assert_eq!(truncate_message("Hello", 5), "Hello");
    }

    #[test]
    fn test_truncate_message_long() {
        assert_eq!(truncate_message("Hello World!", 8), "Hello...");
    }

    #[test]
    fn test_truncate_message_whitespace() {
        assert_eq!(truncate_message("  Hello  ", 10), "Hello");
    }

    // ==================== format_tool_message Tests ====================

    #[test]
    fn test_format_tool_message_known_tools() {
        assert_eq!(format_tool_message("Read"), "Reading file...");
        assert_eq!(format_tool_message("Write"), "Writing file...");
        assert_eq!(format_tool_message("Edit"), "Editing file...");
        assert_eq!(format_tool_message("Bash"), "Running command...");
        assert_eq!(format_tool_message("Glob"), "Searching files...");
        assert_eq!(format_tool_message("Grep"), "Searching content...");
        assert_eq!(format_tool_message("Task"), "Running sub-agent...");
        assert_eq!(format_tool_message("WebFetch"), "Fetching web content...");
        assert_eq!(format_tool_message("WebSearch"), "Searching web...");
    }

    #[test]
    fn test_format_tool_message_mcp_tools() {
        assert_eq!(format_tool_message("mcp__github__pr_list"), "Using pr_list...");
        assert_eq!(format_tool_message("mcp__fs__read"), "Using read...");
    }

    #[test]
    fn test_format_tool_message_unknown() {
        assert_eq!(format_tool_message("CustomTool"), "Using CustomTool...");
    }

    // ==================== remove_fields Tests ====================

    #[test]
    fn test_remove_fields_simple() {
        let mut json: Value = serde_json::json!({
            "type": "assistant",
            "timestamp": "2024-01-01",
            "secret": "should_be_removed"
        });
        remove_fields(&mut json, &["secret".to_string()]);
        assert!(json.get("type").is_some());
        assert!(json.get("timestamp").is_some());
        assert!(json.get("secret").is_none());
    }

    #[test]
    fn test_remove_fields_nested() {
        let mut json: Value = serde_json::json!({
            "outer": {
                "inner": {
                    "keep": "yes",
                    "remove": "no"
                }
            }
        });
        remove_fields(&mut json, &["remove".to_string()]);
        assert!(json["outer"]["inner"]["keep"].is_string());
        assert!(json["outer"]["inner"].get("remove").is_none());
    }

    #[test]
    fn test_remove_fields_array() {
        let mut json: Value = serde_json::json!([
            {"keep": "yes", "remove": "no"},
            {"keep": "yes", "remove": "no"}
        ]);
        remove_fields(&mut json, &["remove".to_string()]);
        assert!(json[0]["keep"].is_string());
        assert!(json[0].get("remove").is_none());
        assert!(json[1].get("remove").is_none());
    }

    // ==================== log_path Tests ====================

    #[test]
    fn test_log_path() {
        let path = log_path("my-task", "abc12345-6789");
        assert!(path.to_string_lossy().contains("my-task"));
        assert!(path.to_string_lossy().contains("abc12345.jsonl"));
    }

    #[test]
    fn test_log_path_short_session_id() {
        let path = log_path("task", "abc");
        assert!(path.to_string_lossy().contains("abc.jsonl"));
    }

    // ==================== TranscriptMetrics Default ====================

    #[test]
    fn test_transcript_metrics_default() {
        let metrics = TranscriptMetrics::default();
        assert_eq!(metrics.input_tokens, 0);
        assert_eq!(metrics.output_tokens, 0);
        assert_eq!(metrics.num_turns, 0);
        assert_eq!(metrics.context_window, 0);
        assert!(metrics.summary.is_none());
        assert!(!metrics.completed);
        assert!(metrics.started_at.is_none());
        assert!(metrics.finished_at.is_none());
        assert!(metrics.current_tool.is_none());
    }
}
