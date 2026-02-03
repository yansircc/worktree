use crate::error::Result;
use crate::models::TaskStatus;
use crate::services::{hooks::HooksEngine, multiplexer::create_multiplexer, TaskContext};

pub fn execute(task_ref: String) -> Result<()> {
    let mut ctx = TaskContext::load(&task_ref)?;

    // Validate (check scratch first to give better error message)
    ctx.require_not_scratch("resumed")?;
    ctx.store.ensure_exists(ctx.name())?;
    ctx.require_status(&[TaskStatus::Idle], "resume")?;
    ctx.require_worktree()?;

    let instance = ctx.require_instance()?.clone();

    // Build hook context and execute
    let hook_ctx = ctx
        .build_hook_context()?
        .with_status("active")
        .with_prev_status("idle");

    let hooks = HooksEngine::new(&ctx.config);
    hooks.execute("resume", &hook_ctx)?;

    // Restart multiplexer window if closed
    let mux = create_multiplexer(instance.multiplexer_type());

    if !mux.window_exists(&instance.session_name, &instance.window_name) {
        // Ensure session exists
        if !mux.session_exists(&instance.session_name) {
            mux.create_session(&instance.session_name)?;
        }

        // Get start_args from config and build command
        let start_args = ctx.config.start_args.replace("${task}", ctx.name());
        let claude_cmd = format!("{} {}", ctx.config.claude_command, start_args);

        // Create new window
        mux.create_window(
            &instance.session_name,
            &instance.window_name,
            &instance.worktree_path,
            &claude_cmd,
        )?;
        println!(
            "Restarted {} window {}:{}",
            ctx.config.multiplexer, instance.session_name, instance.window_name
        );
    } else {
        println!(
            "{} window {}:{} is still alive",
            ctx.config.multiplexer, instance.session_name, instance.window_name
        );
    }

    // Update status to Active
    ctx.set_status(TaskStatus::Active);
    ctx.save_status()?;

    println!("Task '{}' resumed.", ctx.name());
    Ok(())
}
