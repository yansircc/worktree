use crate::error::Result;
use crate::models::{IdleReason, TaskStatus};
use crate::services::{hooks::HooksEngine, multiplexer::create_multiplexer, TaskContext};

pub fn execute(task_ref: String) -> Result<()> {
    let mut ctx = TaskContext::load(&task_ref)?;

    // Validate (check scratch first to give better error message)
    ctx.require_not_scratch("marked for review")?;
    ctx.store.ensure_exists(ctx.name())?;
    let prev_status = ctx.status();
    ctx.validate_transition(TaskStatus::Idle)?;

    // Close multiplexer window if still alive
    if let Some(instance) = ctx.instance() {
        let mux = create_multiplexer(instance.multiplexer_type());
        if mux.kill_window_if_exists(&instance.session_name, &instance.window_name)? {
            println!(
                "Closed {} window {}:{}",
                ctx.config.multiplexer, instance.session_name, instance.window_name
            );
        }
    }

    // Build hook context and execute
    let hook_ctx = ctx
        .build_hook_context()?
        .with_status("idle")
        .with_prev_status(prev_status.display_name());

    let hooks = HooksEngine::new(&ctx.config);
    hooks.execute("review", &hook_ctx)?;

    // Update status to Idle with reviewing phase
    ctx.state_mut().to_idle(IdleReason::HumanReview);
    ctx.save_status()?;

    let name = ctx.name();
    println!("Task '{}' marked for review.", name);
    println!("To merge into main, run: wt complete {}", name);
    println!("To resume working, run: wt resume {}", name);
    Ok(())
}
