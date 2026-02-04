use std::collections::HashMap;

use serde::Serialize;

use crate::models::{StepResult, TaskStatus};
use crate::services::git::GitMetrics;

/// Task metrics for status output
#[derive(Serialize)]
pub struct TaskMetrics {
    pub index: usize,
    pub name: String,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_result: Option<StepResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_since: Option<String>,
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
    pub mux_alive: Option<bool>,
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
    pub active: usize,
    pub idle: usize,
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

impl ActionResponse {
    /// Build a successful action response with task state transition info
    pub fn success(
        action: impl Into<String>,
        task_name: impl Into<String>,
        before: TaskStatus,
        after: TaskStatus,
    ) -> Self {
        Self {
            action: action.into(),
            success: true,
            error: None,
            task: Some(TaskInfo::transition(task_name, before, after)),
            available_actions: None,
            unavailable_actions: None,
            command: None,
        }
    }

    /// Build an error response for action failures
    pub fn error(
        action: impl Into<String>,
        error: impl Into<String>,
        task_name: impl Into<String>,
        status: Option<TaskStatus>,
        mux_alive: Option<bool>,
    ) -> Self {
        Self {
            action: action.into(),
            success: false,
            error: Some(error.into()),
            task: Some(TaskInfo {
                name: task_name.into(),
                status,
                status_before: None,
                status_after: None,
                mux_alive,
            }),
            available_actions: None,
            unavailable_actions: None,
            command: None,
        }
    }

    /// Build an error response without task info (for early failures)
    pub fn error_no_task(action: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            success: false,
            error: Some(error.into()),
            task: None,
            available_actions: None,
            unavailable_actions: None,
            command: None,
        }
    }

    /// Build a "task not found" error response
    pub fn task_not_found(action: impl Into<String>, task_name: impl Into<String>) -> Self {
        let name = task_name.into();
        Self {
            action: action.into(),
            success: false,
            error: Some(format!(
                "Task '{}' not found (only active/idle tasks are available)",
                name
            )),
            task: Some(TaskInfo::name_only(&name)),
            available_actions: None,
            unavailable_actions: None,
            command: None,
        }
    }

    /// Build an unknown action error response
    pub fn unknown_action(action: impl Into<String>, task_name: impl Into<String>) -> Self {
        let action_str = action.into();
        Self {
            action: action_str.clone(),
            success: false,
            error: Some(format!("Unknown action: {}", action_str)),
            task: Some(TaskInfo::name_only(task_name)),
            available_actions: None,
            unavailable_actions: None,
            command: None,
        }
    }

    /// Print response as JSON and exit with appropriate code
    pub fn print_and_exit(self) -> ! {
        println!(
            "{}",
            serde_json::to_string_pretty(&self).unwrap_or_default()
        );
        std::process::exit(if self.success { 0 } else { 1 });
    }
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
    pub mux_alive: Option<bool>,
}

impl TaskInfo {
    /// Create TaskInfo with only name
    pub fn name_only(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: None,
            status_before: None,
            status_after: None,
            mux_alive: None,
        }
    }

    /// Create TaskInfo for a state transition
    pub fn transition(
        name: impl Into<String>,
        before: TaskStatus,
        after: TaskStatus,
    ) -> Self {
        Self {
            name: name.into(),
            status: None,
            status_before: Some(before),
            status_after: Some(after),
            mux_alive: None,
        }
    }

    /// Create TaskInfo with current status and mux state
    pub fn with_status(
        name: impl Into<String>,
        status: TaskStatus,
        mux_alive: bool,
    ) -> Self {
        Self {
            name: name.into(),
            status: Some(status),
            status_before: None,
            status_after: None,
            mux_alive: Some(mux_alive),
        }
    }
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
