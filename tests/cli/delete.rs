//! CLI tests for wt delete command (scratch environments only)

use crate::common::*;

#[test]
fn test_delete_nonexistent() {
    let dir = setup_test_repo();

    let (ok, _, stderr) = run_wt(dir.path(), &["delete", "nonexistent"]);

    assert!(!ok);
    assert!(stderr.contains("not found") || stderr.contains("not a scratch"));
}

#[test]
fn test_delete_normal_task_fails() {
    let dir = setup_repo_with_tasks(&[("task1", &[], "running")]);

    let (ok, _, stderr) = run_wt(dir.path(), &["delete", "task1"]);

    assert!(!ok);
    assert!(
        stderr.contains("not a scratch")
            || stderr.contains("Use 'wt merge")
            || stderr.contains("Use 'wt reset"),
        "Expected error about task not being scratch, got: {}",
        stderr
    );
}

#[test]
fn test_delete_scratch_running() {
    let dir = setup_test_repo();
    set_scratch_status(dir.path(), "s1", "running");

    let (ok, stdout, _) = run_wt(dir.path(), &["delete", "s1"]);

    assert!(ok);
    assert!(
        stdout.contains("deleted") || stdout.contains("Deleting"),
        "Expected delete confirmation, got: {}",
        stdout
    );
}

#[test]
fn test_delete_scratch_review() {
    let dir = setup_test_repo();
    set_scratch_status(dir.path(), "s1", "review");

    let (ok, stdout, _) = run_wt(dir.path(), &["delete", "s1"]);

    assert!(ok);
    assert!(
        stdout.contains("deleted") || stdout.contains("Deleting"),
        "Expected delete confirmation, got: {}",
        stdout
    );
}

#[test]
fn test_delete_scratch_completed_fails() {
    let dir = setup_test_repo();
    set_scratch_status(dir.path(), "s1", "completed");

    let (ok, _, stderr) = run_wt(dir.path(), &["delete", "s1"]);

    assert!(!ok);
    assert!(
        stderr.contains("Invalid") || stderr.contains("transition"),
        "Expected state transition error, got: {}",
        stderr
    );
}
