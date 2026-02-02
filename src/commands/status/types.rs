use std::collections::HashMap;

use serde::Serialize;

use crate::models::TaskStatus;
use crate::services::git::GitMetrics;

/// Task metrics for status output
#[derive(Serialize)]
pub struct TaskMetrics {
    pub index: usize,
    pub name: String,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_human: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_percent: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_tool: Option<String>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub git: Option<GitMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tmux_alive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript_exists: Option<bool>,
}

/// Status output containing all tasks and summary
#[derive(Serialize)]
pub struct StatusOutput {
    pub tasks: Vec<TaskMetrics>,
    pub summary: StatusSummary,
}

/// Summary statistics for status output
#[derive(Serialize)]
pub struct StatusSummary {
    pub running: usize,
    pub done: usize,
    pub total_additions: i32,
    pub total_deletions: i32,
}

/// Action response for --action API
#[derive(Serialize)]
pub struct ActionResponse {
    pub action: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<TaskInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_actions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailable_actions: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<CommandInfo>,
}

/// Task information in action response
#[derive(Serialize)]
pub struct TaskInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TaskStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_before: Option<TaskStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_after: Option<TaskStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tmux_alive: Option<bool>,
}

/// Command information for enter action
#[derive(Serialize, Default)]
pub struct CommandInfo {
    #[serde(rename = "type")]
    pub cmd_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell_command: Option<String>,
}
