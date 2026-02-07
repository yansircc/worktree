use std::env;
use std::path::Path;

use crate::constants::{BRANCH_PREFIX, TASKS_DIR};
use crate::error::{Result, WtError};
use crate::models::{Instance, TaskStatus, TaskStore, WtConfig};
use crate::services::{git, tmux, workspace::WorkspaceInitializer};

pub fn execute(name: Option<String>, print_path: bool) -> Result<()> {
    let config = WtConfig::load()?;
    let mut store = TaskStore::load()?;

    // Generate or validate name
    let name = match name {
        Some(n) => {
            TaskStore::validate_task_name(&n)?;
            n
        }
        None => generate_scratch_name(&store),
    };

    // Conflict checks
    // 1. Check if task file exists
    let task_file = Path::new(TASKS_DIR).join(format!("{}.md", name));
    if task_file.exists() {
        return Err(WtError::TaskExists(name));
    }

    // 2. Check if name exists in status.json
    if store.name_exists_in_status(&name) {
        return Err(WtError::InvalidInput(format!(
            "Name '{}' already exists in status.json",
            name
        )));
    }

    // 3. Check if branch exists (simple name for scratch)
    let branch = format!("{}{}", BRANCH_PREFIX, name);
    if git::branch_exists(&branch) {
        return Err(WtError::BranchExists(branch));
    }

    // Create resources (similar to start.rs but without claude command)
    let cwd = env::current_dir().map_err(|e| WtError::Git(e.to_string()))?;
    let worktree_path = cwd
        .join(&config.worktree_dir)
        .join(&name)
        .to_string_lossy()
        .to_string();

    // Create worktree and branch
    git::create_worktree(&branch, &worktree_path)?;

    // Initialize workspace
    let initializer = WorkspaceInitializer::new(&worktree_path, &cwd);

    // Copy files from main project to worktree
    let copied = initializer.copy_files(&config.copy_files)?;
    for file in &copied {
        if !print_path {
            println!("  Copied: {}", file);
        }
    }

    // Create symlink for status.json so wt commands work from worktree
    initializer.link_status_file()?;

    // Create tmux session with just init_script (or empty command for shell)
    // Each scratch environment gets its own session: project-name
    let cmd = match &config.init_script {
        Some(script) => script.clone(),
        None => String::new(),
    };

    let task_session = format!("{}-{}", &config.tmux_session, &name);
    tmux::create_session_with_command(&task_session, &worktree_path, &cmd)?;

    // Update status.json with scratch=true
    store.set_status(&name, TaskStatus::Running);
    store.set_scratch(&name, true);
    store.set_instance(
        &name,
        Some(Instance {
            branch: branch.clone(),
            worktree_path: worktree_path.clone(),
            tmux_session: task_session.clone(),
            tmux_window: name.clone(), // Keep for backwards compat
            session_id: None, // No Claude session
        }),
    );
    store.save_status()?;

    let relative_path = format!("{}/{}", config.worktree_dir, name);

    if print_path {
        // Only output the path for shell integration
        println!("{}", relative_path);
    } else {
        if config.init_script.is_some() {
            println!("  Init script will run in tmux session");
        }
        println!("Created scratch environment '{}'", name);
        println!("  Worktree: {}", relative_path);
        println!("  Branch:   {}", branch);
        println!("  Tmux:     {}", task_session);
    }

    Ok(())
}

/// Generate next available scratch name: s1, s2, s3...
fn generate_scratch_name(store: &TaskStore) -> String {
    let mut n = 1;
    loop {
        let name = format!("s{}", n);
        let branch = format!("{}{}", BRANCH_PREFIX, name);
        // Check if name exists in status.json or as a branch
        if !store.name_exists_in_status(&name) && !git::branch_exists(&branch) {
            return name;
        }
        n += 1;
    }
}
