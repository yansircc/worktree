//! Pause command - pause a running task (Active -> Idle).

use crate::error::{Result, WtError};
use crate::models::{
    WtConfig, IdleReason, StatusStore, TaskStatus, TaskStore,
};
use crate::services::multiplexer::create_multiplexer;

/// Execute `wt pause <task>`
pub fn execute(task_ref: String, reason: String) -> Result<()> {
    let store = TaskStore::load()?;

    // Resolve task reference
    let name = store.resolve_task_ref(&task_ref)?;

    // Check task exists
    store.ensure_exists(&name)?;

    // Load status
    let mut status = StatusStore::load()?;
    let state = status.get(&name);

    // Check if task is Active (can be paused)
    if state.status != TaskStatus::Active {
        return Err(WtError::InvalidStateTransition {
            from: state.status.display_name().to_string(),
            to: "idle (paused)".to_string(),
        });
    }

    // Parse reason
    let idle_reason = match reason.as_str() {
        "manual" => IdleReason::Manual,
        "done" => IdleReason::Done,
        "human_review" | "idle" => IdleReason::HumanReview,
        "error" => IdleReason::Error,
        "timeout" => IdleReason::Timeout,
        _ => IdleReason::Manual,
    };

    // Try to close multiplexer window
    if let Some(instance) = store.get_instance(&name) {
        let config = WtConfig::load().unwrap_or_default();
        let mux = create_multiplexer(instance.multiplexer_type());

        if mux.kill_window_if_exists(&instance.session_name, &instance.window_name)? {
            println!(
                "Closed {} window {}:{}",
                config.multiplexer, instance.session_name, instance.window_name
            );
        }
    }

    // Update status
    let state = status.get_mut(&name);
    state.to_idle(idle_reason.clone());
    status.save()?;

    println!(
        "Task '{}' paused (reason: {}).",
        name,
        idle_reason.display_name()
    );
    println!("To resume, run: wt resume {}", name);

    Ok(())
}
