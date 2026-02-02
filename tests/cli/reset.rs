//! CLI tests for wt reset command

use crate::common::*;

#[test]
fn test_reset_nonexistent() {
    let dir = setup_test_repo();

    let (ok, _stdout, stderr) = run_wt(dir.path(), &["reset", "nonexistent"]);

    assert!(!ok);
    assert!(stderr.contains("not found"));
}

#[test]
fn test_reset_pending_task_is_idempotent() {
    let dir = setup_repo_with_tasks(&[("task1", &[], "pending")]);

    let (ok, stdout, _stderr) = run_wt(dir.path(), &["reset", "task1"]);

    assert!(ok);
    assert!(stdout.contains("already pending"));
}

#[test]
fn test_reset_running_task() {
    let dir = setup_repo_with_tasks(&[("task1", &[], "running")]);

    let (ok, stdout, _stderr) = run_wt(dir.path(), &["reset", "task1"]);

    assert!(ok);
    assert!(stdout.contains("reset to pending"));

    // Verify status changed to pending
    let (ok, stdout, _) = run_wt(dir.path(), &["list", "--json"]);
    assert!(ok);
    assert!(
        stdout.contains("\"status\": \"pending\"") || stdout.contains("\"status\":\"pending\"")
    );
}

#[test]
fn test_reset_review_task() {
    let dir = setup_repo_with_tasks(&[("task1", &[], "review")]);

    let (ok, stdout, _stderr) = run_wt(dir.path(), &["reset", "task1"]);

    assert!(ok);
    assert!(stdout.contains("reset to pending"));
}

#[test]
fn test_reset_with_non_pending_dependents_fails() {
    // Create task1 and task2 where task2 depends on task1
    // If task2 is running, we should not be able to reset task1
    let dir = setup_repo_with_tasks(&[("task1", &[], "running"), ("task2", &["task1"], "running")]);

    let (ok, _stdout, stderr) = run_wt(dir.path(), &["reset", "task1"]);

    assert!(!ok);
    assert!(stderr.contains("Cannot reset"));
    assert!(stderr.contains("task2"));
    assert!(stderr.contains("depends on it"));
}

#[test]
fn test_reset_with_pending_dependents_succeeds() {
    // If dependent task is pending, reset should succeed
    let dir = setup_repo_with_tasks(&[("task1", &[], "running"), ("task2", &["task1"], "pending")]);

    let (ok, stdout, _stderr) = run_wt(dir.path(), &["reset", "task1"]);

    assert!(ok);
    assert!(stdout.contains("reset to pending"));
}

#[test]
fn test_reset_completed_task() {
    let dir = setup_repo_with_tasks(&[("task1", &[], "completed")]);

    let (ok, stdout, _stderr) = run_wt(dir.path(), &["reset", "task1"]);

    assert!(ok);
    assert!(stdout.contains("reset to pending"));
}
