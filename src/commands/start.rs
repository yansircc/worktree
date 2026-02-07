use std::env;

use uuid::Uuid;

use crate::constants::branch_name;
use crate::error::{Result, WtError};
use crate::models::{Instance, TaskStatus, TaskStore, WtConfig};
use crate::services::{dependency, git, tmux, workspace::WorkspaceInitializer};

pub fn execute(task_ref: Option<String>, all: bool) -> Result<()> {
    if all {
        execute_all()
    } else {
        let task_ref = task_ref.ok_or_else(|| {
            WtError::InvalidInput("Task name or index required (or use --all to start all ready tasks)".into())
        })?;

        // Resolve task reference (name or index) to actual name
        let store = TaskStore::load()?;
        let name = store.resolve_task_ref(&task_ref)?;
        // NOTE: Explicitly drop store because execute_single loads it again.
        // This is necessary since execute_single needs full control over store lifecycle
        // (including modifying status and saving). Future refactor could have execute_single
        // accept &mut TaskStore instead.
        drop(store);

        execute_single(name)
    }
}

/// Start all tasks that are ready (pending with all dependencies merged)
fn execute_all() -> Result<()> {
    let store = TaskStore::load()?;
    let tasks = store.list();

    // Find all ready tasks (pending with all deps merged/archived)
    let ready_tasks: Vec<String> = tasks
        .iter()
        .filter(|task| {
            if store.get_status(task.name()) != TaskStatus::Pending {
                return false;
            }
            task.depends().iter().all(|dep| {
                let status = store.get_status(dep);
                status == TaskStatus::Merged || status == TaskStatus::Archived
            })
        })
        .map(|task| task.name().to_string())
        .collect();

    if ready_tasks.is_empty() {
        println!("No tasks ready to start.");
        println!("Use 'wt next' to see blocked tasks.");
        return Ok(());
    }

    println!("Starting {} task(s)...\n", ready_tasks.len());

    let mut started = 0;
    let mut failed = 0;

    for task_name in ready_tasks {
        print!("Starting '{}'... ", task_name);
        match execute_single(task_name.clone()) {
            Ok(()) => {
                started += 1;
            }
            Err(e) => {
                println!("FAILED: {}", e);
                failed += 1;
            }
        }
    }

    println!("\nSummary: {} started, {} failed", started, failed);

    if started > 0 {
        println!("\nUse 'wt status' to monitor tasks.");
    }

    Ok(())
}

/// Start a single task
fn execute_single(name: String) -> Result<()> {
    let config = WtConfig::load()?;
    let mut store = TaskStore::load()?;

    // Check task exists
    store.ensure_exists(&name)?;

    // Check status from StatusStore
    if store.get_status(&name) == TaskStatus::Running {
        return Err(WtError::AlreadyRunning(name.clone()));
    }

    dependency::check_dependencies_merged(&store, &name)?;

    // Generate session ID for branch naming and Claude Code tracking
    let session_id = Uuid::new_v4().to_string();
    let branch = branch_name(&name, &session_id);
    let cwd = env::current_dir().map_err(|e| WtError::Git(e.to_string()))?;
    let worktree_path = cwd
        .join(&config.worktree_dir)
        .join(&name)
        .to_string_lossy()
        .to_string();

    // Check if branch already exists (e.g., from a previous failed cleanup)
    if git::branch_exists(&branch) {
        return Err(WtError::BranchExists(branch));
    }

    git::create_worktree(&branch, &worktree_path)?;

    // Initialize workspace
    let initializer = WorkspaceInitializer::new(&worktree_path, &cwd);

    // Copy files from main project to worktree
    let copied = initializer.copy_files(&config.copy_files)?;
    for file in &copied {
        println!("  Copied: {}", file);
    }

    // Build agent command: claude_command + start_args
    let expanded_args = config
        .start_args
        .replace("${task}", &name)
        .replace("${branch}", &branch)
        .replace("${worktree}", &worktree_path);

    // Add --session-id to the command for session tracking
    let agent_cmd = format!(
        "{} {} --session-id {}",
        config.claude_command, expanded_args, session_id
    );

    // Build full command: init_script && agent_cmd (if init_script configured)
    let full_cmd = match &config.init_script {
        Some(script) => format!("({}) && {}", script, agent_cmd),
        None => agent_cmd,
    };

    // Each task gets its own tmux session: project-task
    let task_session = format!("{}-{}", &config.tmux_session, &name);
    tmux::create_session_with_command(&task_session, &worktree_path, &full_cmd)?;

    if config.init_script.is_some() {
        println!("  Init script will run in tmux window");
    }

    // Update status in StatusStore
    store.set_status(&name, TaskStatus::Running);
    store.set_instance(
        &name,
        Some(Instance {
            branch: branch.clone(),
            worktree_path: worktree_path.clone(),
            tmux_session: task_session.clone(),
            tmux_window: name.clone(), // Keep for backwards compat, not used for new sessions
            session_id: Some(session_id),
        }),
    );
    store.save_status()?;

    let relative_path = format!("{}/{}", config.worktree_dir, name);

    println!("OK");
    println!("  Worktree: {}", relative_path);
    println!("  Branch:   {}", branch);

    Ok(())
}
