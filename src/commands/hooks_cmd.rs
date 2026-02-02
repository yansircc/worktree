//! Hooks command - manually trigger hooks for debugging.

use crate::error::Result;
use crate::models::{WtConfig, TaskStore};
use crate::services::git;
use crate::services::hooks::{ExecutionContext, HooksEngine};

/// Execute `wt hooks run <hook>`
pub fn execute_run(hook: String, task: Option<String>) -> Result<()> {
    // Try to load v2 config, fallback to error
    let config = WtConfig::load()?;

    // Build execution context
    let context = if let Some(task_name) = task {
        let store = TaskStore::load()?;

        // Resolve task reference
        let name = store.resolve_task_ref(&task_name)?;

        // Get instance info if available
        let repo_root = git::get_repo_root().unwrap_or_else(|_| ".".to_string());

        if let Some(instance) = store.get_instance(&name) {
            ExecutionContext::new(&name, &instance.branch, &instance.worktree_path, &repo_root)
                .with_session(&instance.session_name)
                .with_window(&instance.window_name)
        } else {
            // Task exists but not started
            let worktree_path = format!("{}/{}", config.worktree_dir, name);
            let branch = format!("wt/{}", name);
            ExecutionContext::new(&name, &branch, &worktree_path, &repo_root)
                .with_session(&config.session_name)
                .with_window(&name)
        }
    } else {
        // No task specified, use minimal context
        let repo_root = git::get_repo_root().unwrap_or_else(|_| ".".to_string());
        ExecutionContext::new("", "", "", &repo_root)
            .with_session(&config.session_name)
    };

    // Create engine and execute
    let engine = HooksEngine::new(&config);

    if !engine.has_hook(&hook) {
        println!("Hook '{}' is not defined in config.jsonc", hook);
        println!("Available hooks: run, review, resume, complete, delete, reset");
        return Ok(());
    }

    println!("Executing hook: {}", hook);
    engine.execute(&hook, &context)?;

    println!("Hook '{}' completed successfully.", hook);
    Ok(())
}

/// Execute `wt hooks list`
pub fn execute_list() -> Result<()> {
    let config = match WtConfig::load() {
        Ok(c) => c,
        Err(_) => {
            println!("No .wt/config.jsonc found. Using defaults.");
            println!();
            println!("Available hooks (none configured):");
            println!("  run      - triggered by 'wt run'");
            println!("  review   - triggered by 'wt review'");
            println!("  resume   - triggered by 'wt resume'");
            println!("  complete - triggered by 'wt complete'");
            println!("  delete   - triggered by 'wt delete'");
            println!("  reset    - triggered by 'wt reset'");
            return Ok(());
        }
    };

    println!("Hooks from .wt/config.jsonc:");
    println!();

    let hooks = [
        ("run", "triggered by 'wt run'"),
        ("review", "triggered by 'wt review'"),
        ("resume", "triggered by 'wt resume'"),
        ("complete", "triggered by 'wt complete'"),
        ("delete", "triggered by 'wt delete'"),
        ("reset", "triggered by 'wt reset'"),
    ];

    for (name, desc) in hooks {
        let status = if config.get_hook(name).is_some() {
            "configured"
        } else {
            "not configured"
        };
        println!("  {:10} - {} [{}]", name, desc, status);
    }

    Ok(())
}

/// Dispatch hooks subcommand
pub fn execute(action: crate::cli::HooksAction) -> Result<()> {
    match action {
        crate::cli::HooksAction::Run { hook, task } => execute_run(hook, task),
        crate::cli::HooksAction::List => execute_list(),
    }
}
