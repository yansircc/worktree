//! Step command for Phases v2.
//!
//! Used by Agents to mark the current step's status:
//! - `wt step done [message]` - Mark step as successful
//! - `wt step block [message]` - Mark step as blocked (needs human intervention)
//! - `wt step fail [message]` - Mark step as failed
//!
//! Auto-discovery:
//! - Detects if running in a worktree and finds main repo automatically
//! - Extracts task name from branch name (wt/<task>-<hash>)
//!
//! Environment variables (optional, for override):
//! - WT_TASK: Override task name detection
//! - WT_PHASE: Current phase (for logging)
//! - WT_STEP: Current step index (for logging)

use std::fs::{self, OpenOptions};
use std::io::Write;

use chrono::Utc;

use crate::error::{Result, WtError};
use crate::models::{StepResult, StatusStore, TaskStatus};
use crate::models::workflow::WorkflowState;
use crate::services::{git, transcript};
use crate::services::observer::log::WorkflowLogEntry;

/// Step action type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepAction {
    Done,
    Block,
    Fail,
}

impl StepAction {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "done" => Some(StepAction::Done),
            "block" => Some(StepAction::Block),
            "fail" => Some(StepAction::Fail),
            _ => None,
        }
    }
}

/// Execute the step command
///
/// # Arguments
/// * `action` - The action to perform (done, block, fail)
/// * `message` - Optional message (for block/fail)
pub fn execute(action: &str, message: Option<String>) -> Result<()> {
    let action_type = StepAction::from_str(action)
        .ok_or_else(|| WtError::InvalidInput(format!(
            "Unknown step action '{}'. Valid actions: done, block, fail",
            action
        )))?;

    // Get task name: try env var first, then auto-detect from branch
    // IMPORTANT: Must detect branch BEFORE changing to repo root
    let task_name = match std::env::var("WT_TASK") {
        Ok(name) => name,
        Err(_) => detect_task_from_branch()?,
    };

    // Auto-discover repo root (works from worktree)
    let repo_root = git::get_repo_root()?;

    // Change to repo root to operate on the correct status.json
    std::env::set_current_dir(&repo_root).map_err(|e| {
        WtError::Io {
            operation: "change to repo root".to_string(),
            path: repo_root.clone(),
            message: e.to_string(),
        }
    })?;

    // Load status store (now from repo root)
    let mut status = StatusStore::load()?;

    // Get phase from status store (environment variable as override)
    let phase = std::env::var("WT_PHASE")
        .ok()
        .or_else(|| status.get_phase(&task_name).map(|p| p.to_string()));
    let step_index = std::env::var("WT_STEP").ok();

    // Get current task state
    let state = status.get_mut(&task_name);

    // Idempotent: if already in the target state, succeed silently
    if state.status == TaskStatus::Idle {
        match (&action_type, &state.step_result) {
            (StepAction::Done, Some(StepResult::Done)) => {
                println!("Task '{}' already marked as done", task_name);
                return Ok(());
            }
            (StepAction::Block, Some(StepResult::HumanReview)) => {
                println!("Task '{}' already marked as blocked", task_name);
                return Ok(());
            }
            (StepAction::Fail, Some(StepResult::Error)) => {
                println!("Task '{}' already marked as failed", task_name);
                return Ok(());
            }
            _ => {}
        }
    }

    // Validate current status - must be Active (or Idle for state change)
    if state.status != TaskStatus::Active && state.status != TaskStatus::Idle {
        return Err(WtError::InvalidStateTransition {
            from: state.status.display_name().to_string(),
            to: "step transition".to_string(),
        });
    }

    // Log step action
    log_step_action(&task_name, phase.as_deref(), step_index.as_deref(), &action_type, message.as_deref());

    // Save message for inter-step communication if provided
    if let Some(ref msg) = message {
        save_step_message(&task_name, phase.as_deref(), &action_type, msg);
    }

    // Apply action
    match action_type {
        StepAction::Done => {
            // Step completed successfully
            state.to_idle(StepResult::Done);
            println!("Step completed successfully for task '{}'", task_name);
        }
        StepAction::Block => {
            // Step blocked - needs human intervention
            state.to_idle(StepResult::HumanReview);
            if let Some(msg) = &message {
                println!("Step blocked for task '{}': {}", task_name, msg);
            } else {
                println!("Step blocked for task '{}' (needs human intervention)", task_name);
            }
        }
        StepAction::Fail => {
            // Step failed
            state.to_idle(StepResult::Error);
            if let Some(msg) = &message {
                println!("Step failed for task '{}': {}", task_name, msg);
            } else {
                println!("Step failed for task '{}'", task_name);
            }
        }
    }

    // Save status
    status.save()?;

    // Create symlink to agent transcript in log directory
    link_agent_transcript(&task_name, &status, phase.as_deref(), step_index.as_deref());

    // Update context.json to include agent step result
    update_workflow_context(&task_name, phase.as_deref(), &action_type);

    Ok(())
}

/// Create a symlink from the log directory to the agent's transcript file.
fn link_agent_transcript(
    task_name: &str,
    status: &StatusStore,
    phase: Option<&str>,
    step_index: Option<&str>,
) {
    let phase_str = match phase {
        Some(p) => p,
        None => return,
    };

    // Find instance to get worktree path
    let instance = match status.get_instance(task_name) {
        Some(inst) => inst,
        None => return,
    };

    let worktree_path = match instance.worktree_path.as_deref() {
        Some(p) => p,
        None => return,
    };

    // Find the latest transcript for this worktree
    let transcript_path = match transcript::find_latest_transcript(worktree_path) {
        Some(p) => p,
        None => return,
    };

    // Build symlink path: .wt/logs/<task>/<phase>/step-<N>-agent.jsonl
    let log_dir = format!(".wt/logs/{}/{}", task_name, phase_str);
    let link_name = match step_index {
        Some(idx) => format!("step-{}-agent.jsonl", idx),
        None => "agent-transcript.jsonl".to_string(),
    };
    let link_path = std::path::PathBuf::from(&log_dir).join(&link_name);

    // Remove existing symlink if present
    if link_path.exists() || link_path.symlink_metadata().is_ok() {
        let _ = std::fs::remove_file(&link_path);
    }

    // Create parent directory if needed
    let _ = std::fs::create_dir_all(&log_dir);

    // Create symlink
    #[cfg(unix)]
    {
        let _ = std::os::unix::fs::symlink(&transcript_path, &link_path);
    }
}

/// Detect task name from current git branch
///
/// Branch format: wt/<task-name>-<hash>
/// Returns the task name portion
fn detect_task_from_branch() -> Result<String> {
    let cwd = std::env::current_dir()
        .map_err(|e| WtError::Io {
            operation: "get current dir".to_string(),
            path: ".".to_string(),
            message: e.to_string(),
        })?;

    let branch = git::current_branch(cwd.to_string_lossy().as_ref())?;

    // Parse branch name: wt/<task>-<hash>
    if let Some(rest) = branch.strip_prefix("wt/") {
        // Find the last dash followed by hex hash (6+ chars)
        // e.g., "my-task-abc123" -> "my-task"
        if let Some(pos) = find_hash_separator(rest) {
            return Ok(rest[..pos].to_string());
        }
        // No hash suffix, use the whole name
        return Ok(rest.to_string());
    }

    Err(WtError::InvalidInput(format!(
        "Cannot detect task from branch '{}'. \
         Expected format: wt/<task>-<hash>. \
         Set WT_TASK environment variable to override.",
        branch
    )))
}

/// Find the position of the last '-' before a hex hash suffix
fn find_hash_separator(s: &str) -> Option<usize> {
    // Look for pattern: -<hex>{6,} at the end
    for (i, _) in s.rmatch_indices('-') {
        let suffix = &s[i + 1..];
        if suffix.len() >= 6 && suffix.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(i);
        }
    }
    None
}

/// Log step action to task log file
fn log_step_action(
    task_name: &str,
    phase: Option<&str>,
    step_index: Option<&str>,
    action: &StepAction,
    message: Option<&str>,
) {
    let phase_str = phase.unwrap_or("unknown");
    let log_dir = format!(".wt/logs/{}/{}", task_name, phase_str);
    if fs::create_dir_all(&log_dir).is_err() {
        return;
    }

    let log_path = format!("{}/step-actions.log", log_dir);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        let step_str = step_index.unwrap_or("?");
        let action_str = match action {
            StepAction::Done => "done",
            StepAction::Block => "block",
            StepAction::Fail => "fail",
        };
        let msg_str = message.map(|m| format!(": {}", m)).unwrap_or_default();
        let _ = writeln!(
            file,
            "[{}] step[{}] {} {}",
            timestamp, step_str, action_str, msg_str
        );
    }
}

/// Save step message for inter-step communication
fn save_step_message(task_name: &str, phase: Option<&str>, action: &StepAction, message: &str) {
    let phase_str = phase.unwrap_or("unknown");
    let msg_dir = format!(".wt/logs/{}/{}", task_name, phase_str);
    if fs::create_dir_all(&msg_dir).is_err() {
        return;
    }

    // Save to last-step-result.txt for easy access by subsequent steps
    let result_path = format!("{}/last-step-result.txt", msg_dir);
    let action_str = match action {
        StepAction::Done => "done",
        StepAction::Block => "block",
        StepAction::Fail => "fail",
    };
    let content = format!("action: {}\nmessage: {}\n", action_str, message);
    let _ = fs::write(&result_path, content);
}

/// Update context.json to include the agent step result.
/// Reads the existing context.json (written by workflow executor for script steps),
/// increments step_count, updates success/fail/block counts and final state.
fn update_workflow_context(task_name: &str, phase: Option<&str>, action: &StepAction) {
    let phase_str = match phase {
        Some(p) => p,
        None => return,
    };

    let context_path = format!(".wt/logs/{}/{}/context.json", task_name, phase_str);

    // Read existing context.json
    let mut entry: WorkflowLogEntry = match fs::read_to_string(&context_path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(e) => e,
            Err(_) => return,
        },
        Err(_) => {
            // No existing context.json (no script steps before agent),
            // create a fresh one
            WorkflowLogEntry {
                workflow_id: None,
                workflow_name: "workflow".to_string(),
                state: WorkflowState::Pending,
                step_count: 0,
                steps_succeeded: 0,
                steps_failed: 0,
                steps_skipped: 0,
                duration_ms: 0,
                started_at: Utc::now(),
                ended_at: Utc::now(),
            }
        }
    };

    // Add agent step result
    entry.step_count += 1;
    match action {
        StepAction::Done => {
            entry.steps_succeeded += 1;
            // Only mark success if no previous failures
            if entry.state != WorkflowState::Failed {
                entry.state = WorkflowState::Success;
            }
        }
        StepAction::Block => {
            entry.steps_skipped += 1;
            entry.state = WorkflowState::Blocked;
        }
        StepAction::Fail => {
            entry.steps_failed += 1;
            entry.state = WorkflowState::Failed;
        }
    }

    // Update end time and duration
    entry.ended_at = Utc::now();
    let duration = entry.ended_at.signed_duration_since(entry.started_at);
    entry.duration_ms = duration.num_milliseconds().max(0) as u64;

    // Write back
    if let Ok(json) = serde_json::to_string_pretty(&entry) {
        let _ = fs::write(&context_path, json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_action_from_str() {
        assert_eq!(StepAction::from_str("done"), Some(StepAction::Done));
        assert_eq!(StepAction::from_str("DONE"), Some(StepAction::Done));
        assert_eq!(StepAction::from_str("block"), Some(StepAction::Block));
        assert_eq!(StepAction::from_str("fail"), Some(StepAction::Fail));
        assert_eq!(StepAction::from_str("invalid"), None);
    }

    #[test]
    fn test_find_hash_separator() {
        assert_eq!(find_hash_separator("my-task-abc123"), Some(7));
        assert_eq!(find_hash_separator("task-6c61f2"), Some(4));
        assert_eq!(find_hash_separator("multi-word-task-deadbeef"), Some(15));
        assert_eq!(find_hash_separator("no-hash"), None);
        assert_eq!(find_hash_separator("short-ab"), None); // hash too short
        assert_eq!(find_hash_separator("task"), None); // no dash
    }

    #[test]
    fn test_execute_outside_git_repo() {
        // This test verifies error handling when not in a git repo
        // The auto-discovery will fail
        std::env::remove_var("WT_TASK");

        let result = execute("done", None);
        // Should fail (git detection or status loading)
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_invalid_action() {
        std::env::set_var("WT_TASK", "test-task");

        let result = execute("invalid", None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unknown step action"));

        std::env::remove_var("WT_TASK");
    }
}
