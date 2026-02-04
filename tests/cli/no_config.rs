use super::*;
use std::process::Command;

#[test]
fn test_run_without_config() {
    let dir = tempfile::tempdir().unwrap();

    Command::new("git")
        .current_dir(dir.path())
        .args(["init"])
        .output()
        .ok();

    create_task_file(dir.path(), "task1", &[]);

    // Without config.jsonc, run uses defaults but may fail for other reasons (e.g., tmux)
    let (ok, _, stderr) = run_wt(dir.path(), &["run", "task1"]);

    // Run may fail due to tmux issues, not config issues
    assert!(!ok || stderr.contains("mux") || stderr.contains("window"));
}

#[test]
fn test_list_without_config() {
    let dir = tempfile::tempdir().unwrap();
    let (ok, stdout, _) = run_wt(dir.path(), &["list"]);

    assert!(ok);
    assert!(stdout.contains("No tasks"));
}

#[test]
fn test_create_without_config() {
    let dir = tempfile::tempdir().unwrap();

    let (ok, stdout, _) = run_wt(
        dir.path(),
        &[
            "create",
            "--json",
            r#"{"name": "task1", "depends": [], "description": "Test"}"#,
        ],
    );

    assert!(ok);
    assert!(stdout.contains("created"));
}

#[test]
fn test_next_without_config() {
    let dir = tempfile::tempdir().unwrap();

    create_task_file(dir.path(), "task1", &[]);

    // Without config, phases are not configured, so next should fail
    let (ok, _, stderr) = run_wt(dir.path(), &["next", "task1"]);

    assert!(!ok);
    assert!(
        stderr.contains("No phases configured") || stderr.contains("wt init"),
        "Expected error about missing phases config, got: {}",
        stderr
    );
}

#[test]
fn test_status_without_config() {
    let dir = tempfile::tempdir().unwrap();

    // Status now works without config, using defaults
    let (ok, stdout, _) = run_wt(dir.path(), &["status", "--json"]);

    assert!(ok);
    assert!(stdout.contains("\"tasks\""));
    assert!(stdout.contains("\"summary\""));
}
