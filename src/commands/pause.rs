//! Pause command - pause a running task (Active -> Idle).

use crate::error::Result;
use crate::models::{IdleReason, TaskStatus};
use crate::services::{multiplexer::create_multiplexer, TaskContext};

/// Execute `wt pause <task>`
pub fn execute(task_ref: String, reason: String) -> Result<()> {
    let mut ctx = TaskContext::load(&task_ref)?;

    // Validate
    ctx.store.ensure_exists(ctx.name())?;
    ctx.require_status(&[TaskStatus::Active], "pause")?;

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
    if let Some(instance) = ctx.instance() {
        let mux = create_multiplexer(instance.multiplexer_type());
        if mux.kill_window_if_exists(&instance.session_name, &instance.window_name)? {
            println!(
                "Closed {} window {}:{}",
                ctx.config.multiplexer, instance.session_name, instance.window_name
            );
        }
    }

    // Update status
    ctx.state_mut().to_idle(idle_reason.clone());
    ctx.save_status()?;

    println!(
        "Task '{}' paused (reason: {}).",
        ctx.name(),
        idle_reason.display_name()
    );
    println!("To resume, run: wt resume {}", ctx.name());

    Ok(())
}
