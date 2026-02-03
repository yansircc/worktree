//! Step command for Phases v2.
//!
//! Used by Agents to mark the current step's status:
//! - `wt step done` - Mark step as successful
//! - `wt step block [message]` - Mark step as blocked (needs human intervention)
//! - `wt step fail [message]` - Mark step as failed

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
    let action = StepAction::from_str(action)
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

    // Apply action
    match action {
        StepAction::Done => {
            // Step completed successfully
            // In legacy mode, this just marks the task as Idle with Done reason
            // The agent/process may continue or exit
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
