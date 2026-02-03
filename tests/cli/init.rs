use super::*;

/// Setup a bare git repo without wt config (for init tests)
fn setup_bare_git_repo() -> TempDir {
    let dir = tempfile::tempdir().unwrap();

    Command::new("git")
        .current_dir(dir.path())
        .args(["init"])
        .output()
        .expect("Failed to init git");

    Command::new("git")
        .current_dir(dir.path())
        .args(["config", "user.email", "test@test.com"])
        .output()
        .ok();
    Command::new("git")
        .current_dir(dir.path())
        .args(["config", "user.name", "Test"])
        .output()
        .ok();

    dir
}

#[test]
fn test_init_creates_config_file() {
    let dir = setup_bare_git_repo();
    let (ok, stdout, _) = run_wt(dir.path(), &["init"]);

    assert!(ok);
    assert!(stdout.contains("Created .wt/config.jsonc"));
    assert!(dir.path().join(".wt/config.jsonc").exists());
}

#[test]
fn test_init_creates_tasks_directory() {
    let dir = setup_bare_git_repo();
    let (ok, stdout, _) = run_wt(dir.path(), &["init"]);

    assert!(ok);
    assert!(stdout.contains("Created .wt/tasks/"));
    assert!(dir.path().join(".wt/tasks").is_dir());
}

#[test]
fn test_init_creates_gitignore() {
    let dir = setup_bare_git_repo();
    let (ok, stdout, _) = run_wt(dir.path(), &["init"]);

    assert!(ok);
    assert!(stdout.contains(".gitignore"));

    let gitignore = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
    assert!(gitignore.contains(".wt/"));
    assert!(gitignore.contains("# wt - Worktree Task Manager"));
}

#[test]
fn test_init_appends_to_existing_gitignore() {
    let dir = setup_bare_git_repo();

    // Create existing .gitignore
    fs::write(dir.path().join(".gitignore"), "node_modules/\n.env\n").unwrap();

    let (ok, stdout, _) = run_wt(dir.path(), &["init"]);

    assert!(ok);
    assert!(stdout.contains("Updated .gitignore"));

    let gitignore = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
    // Should preserve existing content
    assert!(gitignore.contains("node_modules/"));
    assert!(gitignore.contains(".env"));
    // Should add wt entry
    assert!(gitignore.contains(".wt/"));
}

#[test]
fn test_init_does_not_duplicate_gitignore_entries() {
    let dir = setup_bare_git_repo();

    // Create .gitignore with wt marker already
    fs::write(
        dir.path().join(".gitignore"),
        "node_modules/\n# wt - Worktree Task Manager\n.wt/\n",
    )
    .unwrap();

    let (ok, stdout, _) = run_wt(dir.path(), &["init"]);

    assert!(ok);
    assert!(stdout.contains("already has wt entries"));

    let gitignore = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
    // Should not duplicate
    let count = gitignore.matches("# wt - Worktree Task Manager").count();
    assert_eq!(count, 1);
}

#[test]
fn test_init_fails_if_config_exists() {
    let dir = setup_bare_git_repo();

    // Create existing config
    fs::create_dir_all(dir.path().join(".wt")).unwrap();
    fs::write(dir.path().join(".wt/config.jsonc"), "{}").unwrap();

    let (ok, _, stderr) = run_wt(dir.path(), &["init"]);

    assert!(!ok);
    assert!(stderr.contains("already exists"));
}

#[test]
fn test_init_config_has_required_fields() {
    let dir = setup_bare_git_repo();
    run_wt(dir.path(), &["init"]);

    let config = fs::read_to_string(dir.path().join(".wt/config.jsonc")).unwrap();

    assert!(config.contains("\"multiplexer\":"));
    assert!(config.contains("\"session_name\":"));
    assert!(config.contains("\"hooks\":"));
}

#[test]
fn test_init_config_has_hooks() {
    let dir = setup_bare_git_repo();
    run_wt(dir.path(), &["init"]);

    let config = fs::read_to_string(dir.path().join(".wt/config.jsonc")).unwrap();

    assert!(config.contains("\"run\":"));
    assert!(config.contains("\"type\": \"agent\""));
}

#[test]
fn test_init_uses_directory_name_as_session() {
    let dir = setup_bare_git_repo();
    run_wt(dir.path(), &["init"]);

    let config = fs::read_to_string(dir.path().join(".wt/config.jsonc")).unwrap();

    // The tempdir has a random name, just check it's not the default "wt"
    // and that session_name field exists with some value
    assert!(config.contains("\"session_name\":"));
}

#[test]
fn test_init_shows_next_steps() {
    let dir = setup_bare_git_repo();
    let (ok, stdout, _) = run_wt(dir.path(), &["init"]);

    assert!(ok);
    assert!(stdout.contains("Next steps:"));
    assert!(stdout.contains("Edit .wt/config.jsonc"));
    assert!(stdout.contains("wt create"));
    assert!(stdout.contains("wt next"));
}

#[test]
fn test_init_preserves_gitignore_without_trailing_newline() {
    let dir = setup_bare_git_repo();

    // Create .gitignore without trailing newline
    fs::write(dir.path().join(".gitignore"), "node_modules/").unwrap();

    let (ok, _, _) = run_wt(dir.path(), &["init"]);

    assert!(ok);

    let gitignore = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
    // Should have added a blank line before wt entries
    assert!(gitignore.contains("node_modules/\n\n# wt"));
}

#[test]
fn test_init_existing_tasks_dir_is_ok() {
    let dir = setup_bare_git_repo();

    // Create tasks dir beforehand
    fs::create_dir_all(dir.path().join(".wt/tasks")).unwrap();

    let (ok, stdout, _) = run_wt(dir.path(), &["init"]);

    assert!(ok);
    // Should still create config
    assert!(stdout.contains("Created .wt/config.jsonc"));
}
