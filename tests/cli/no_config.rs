use super::*;
use std::process::Command;

#[test]
fn test_start_without_config() {
    let dir = tempfile::tempdir().unwrap();

    Command::new("git")
        .current_dir(dir.path())
        .args(["init"])
        .output()
        .ok();

    create_task_file(dir.path(), "task1", &[]);

    let (ok, _, stderr) = run_wt(dir.path(), &["start", "task1"]);

    assert!(!ok);
    assert!(stderr.contains("Config") || stderr.contains(".wt.yaml"));
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

    let (ok, stdout, _) = run_wt(dir.path(), &["next"]);

    assert!(ok);
    assert!(stdout.contains("task1"));
}

#[test]
fn test_status_without_config() {
    let dir = tempfile::tempdir().unwrap();

    let (ok, _, stderr) = run_wt(dir.path(), &["status"]);

    assert!(!ok);
    assert!(stderr.contains("Config") || stderr.contains(".wt.yaml"));
}
