use std::path::Path;
use std::process::Command;

use crate::error::{Result, WtError};
use crate::models::{TaskStatus, TaskStore, WtConfig};
use crate::services::multiplexer::create_multiplexer;

pub fn execute(task_ref: String) -> Result<()> {
    let config = WtConfig::load()?;
    let mut store = TaskStore::load()?;

    // Resolve task reference (name or index) to actual name
    let name = store.resolve_task_ref(&task_ref)?;

    // Check if scratch environment
    if store.is_scratch(&name) {
        return Err(WtError::InvalidInput(format!(
            "Scratch environment '{}' cannot be marked for review. Use 'wt delete {}' to clean up.",
            name, name
        )));
    }

    // Check task exists and validate transition
    store.ensure_exists(&name)?;
    store.validate_transition(&name, TaskStatus::Review)?;

    // Close multiplexer window if still alive
    if let Some(instance) = store.get_instance(&name) {
        let mux = create_multiplexer(instance.multiplexer_type());
        if mux.kill_window_if_exists(&instance.session_name, &instance.window_name)? {
            println!("Closed {} window {}:{}", config.multiplexer, instance.session_name, instance.window_name);
        }
    }

    // Run review script if configured
    if let Some(ref script) = config.review_script {
        if let Some(instance) = store.get_instance(&name) {
            let worktree_path = &instance.worktree_path;
            if Path::new(worktree_path).exists() {
                println!("Running review script...");
                let status = Command::new("sh")
                    .arg("-c")
                    .arg(script)
                    .current_dir(worktree_path)
                    .status()
                    .map_err(|e| WtError::Script {
                        script: "review_script".to_string(),
                        message: e.to_string(),
                    })?;

                if !status.success() {
                    return Err(WtError::ReviewScriptFailed(name.clone()));
                }
            }
        }
    }

    store.set_status(&name, TaskStatus::Review);
    store.save_status()?;

    println!("Task '{}' marked for review.", name);
    println!("To merge into main, run: wt merge {}", name);
    println!("To resume working, run: wt resume {}", name);
    Ok(())
}
