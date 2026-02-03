//! Step command for Phases v2.
//!
//! Used by Agents to mark the current step's status:
//! - `wt step done [message]` - Mark step as successful
//! - `wt step block [message]` - Mark step as blocked (needs human intervention)
//! - `wt step fail [message]` - Mark step as failed
//!
//! Environment variables:
//! - WT_TASK: Current task name (required)
//! - WT_PHASE: Current phase (optional, for logging)
//! - WT_STEP: Current step index (optional, for logging)

use std::fs::{self, OpenOptions};
use std::io::Write;

use chrono::Utc;

use crate::error::{Result, WtError};
use crate::models::{IdleReason, StatusStore, TaskStatus};

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

    // Get current task from environment variable
    let task_name = std::env::var("WT_TASK").map_err(|_| {
        WtError::InvalidInput(
            "WT_TASK environment variable not set. \
             This command should be run from within a wt task context."
                .to_string(),
        )
    })?;

    // Get optional context from environment
    let phase = std::env::var("WT_PHASE").ok();
    let step_index = std::env::var("WT_STEP").ok();

    // Load status store
    let mut status = StatusStore::load()?;

    // Get current task state
    let state = status.get_mut(&task_name);

    // Validate current status - must be Active
    if state.status != TaskStatus::Active {
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
            state.to_idle(IdleReason::Done);
            println!("Step completed successfully for task '{}'", task_name);
        }
        StepAction::Block => {
            // Step blocked - needs human intervention
            state.to_idle(IdleReason::HumanReview);
            if let Some(msg) = &message {
                println!("Step blocked for task '{}': {}", task_name, msg);
            } else {
                println!("Step blocked for task '{}' (needs human intervention)", task_name);
            }
        }
        StepAction::Fail => {
            // Step failed
            state.to_idle(IdleReason::Error);
            if let Some(msg) = &message {
                println!("Step failed for task '{}': {}", task_name, msg);
            } else {
                println!("Step failed for task '{}'", task_name);
            }
        }
    }

    // Save status
    status.save()?;

    Ok(())
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
    fn test_execute_without_wt_task() {
        // This test verifies error handling when WT_TASK is not set
        // Note: In CI/test environment, WT_TASK might be set or status file might exist
        // So we just verify that execute returns an error (any error is acceptable)
        std::env::remove_var("WT_TASK");

        let result = execute("done", None);
        // Either WT_TASK missing error or status loading error is acceptable
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
