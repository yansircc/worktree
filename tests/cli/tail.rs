//! CLI tests for wt tail command
//!
//! The tail command shows the last N assistant messages from a task's transcript.
//! These tests verify error handling since actual transcripts require real Claude sessions.
//!
//! Note: wt tail uses -n for count, e.g., `wt tail task1 -n 5`

use crate::common::*;
use serde_json::json;

// ==================== Error Cases ====================

#[test]
fn test_tail_nonexistent_task() {
    let dir = setup_test_repo();

    let (ok, _, stderr) = run_wt(dir.path(), &["tail", "nonexistent"]);

    assert!(!ok);
    assert!(
        stderr.contains("not found"),
        "Expected not found error, got: {}",
        stderr
    );
}

#[test]
fn test_tail_pending_task_error() {
    let dir = setup_repo_with_tasks(&[("task1", &[], "pending")]);

    let (ok, _, stderr) = run_wt(dir.path(), &["tail", "task1"]);

    assert!(!ok);
    // Error message is "has not been started"
    assert!(
        stderr.contains("not") && stderr.contains("started"),
        "Expected 'not started' error, got: {}",
        stderr
    );
}

#[test]
fn test_tail_running_task_without_transcript() {
    let dir = setup_test_repo();

    // Create task with instance but no transcript
    create_task_file(dir.path(), "task1", &[]);
    set_task_status_with_instance(
        dir.path(),
        "task1",
        "running",
        Some(json!({
            "branch": "wt/task1",
            "worktree_path": "/tmp/nonexistent-worktree",
            "session_name": "test-session",
            "window_name": "task1"
        })),
    );

    let (ok, _, stderr) = run_wt(dir.path(), &["tail", "task1"]);

    assert!(!ok);
    // Error message mentions "worktree no longer exists"
    assert!(
        stderr.contains("worktree") || stderr.contains("transcript") || stderr.contains("Worktree"),
        "Expected worktree/transcript error, got: {}",
        stderr
    );
}

#[test]
fn test_tail_review_task_without_transcript() {
    let dir = setup_test_repo();

    create_task_file(dir.path(), "task1", &[]);
    set_task_status_with_instance(
        dir.path(),
        "task1",
        "review",
        Some(json!({
            "branch": "wt/task1",
            "worktree_path": "/tmp/nonexistent-worktree",
            "session_name": "test-session",
            "window_name": "task1"
        })),
    );

    let (ok, _, stderr) = run_wt(dir.path(), &["tail", "task1"]);

    assert!(!ok);
    // Should fail gracefully with worktree/transcript error
    assert!(
        stderr.contains("worktree") || stderr.contains("transcript"),
        "Expected worktree/transcript error, got: {}",
        stderr
    );
}

// ==================== Count Parameter ====================

#[test]
fn test_tail_with_count_parameter() {
    let dir = setup_test_repo();

    create_task_file(dir.path(), "task1", &[]);
    set_task_status_with_instance(
        dir.path(),
        "task1",
        "running",
        Some(json!({
            "branch": "wt/task1",
            "worktree_path": "/tmp/nonexistent-worktree",
            "session_name": "test-session",
            "window_name": "task1"
        })),
    );

    // Use -n for count parameter
    let (ok, _, stderr) = run_wt(dir.path(), &["tail", "task1", "-n", "5"]);

    // Will fail due to missing transcript, not invalid argument
    assert!(!ok);
    assert!(
        !stderr.contains("Invalid argument") && !stderr.contains("unexpected"),
        "Should accept -n parameter, got: {}",
        stderr
    );
}

#[test]
fn test_tail_default_count_is_one() {
    let dir = setup_test_repo();

    // Test that help shows default count
    let (ok, stdout, _) = run_wt(dir.path(), &["tail", "--help"]);

    assert!(ok);
    // Help mentions -n and default value 1
    assert!(
        stdout.contains("-n") && stdout.contains("1"),
        "Help should mention -n and default count 1, got: {}",
        stdout
    );
}

// ==================== Task Without Instance ====================

#[test]
fn test_tail_task_without_instance() {
    let dir = setup_repo_with_tasks(&[("task1", &[], "review")]);

    // Task is in review but has no instance info
    let (ok, _, stderr) = run_wt(dir.path(), &["tail", "task1"]);

    assert!(!ok);
    assert!(
        stderr.contains("not found")
            || stderr.contains("instance")
            || stderr.contains("transcript"),
        "Expected error about missing instance/transcript, got: {}",
        stderr
    );
}

// ==================== Scratch Task ====================

#[test]
fn test_tail_scratch_task() {
    let dir = setup_test_repo();

    // Create scratch task file too (tail uses ensure_exists which checks task file)
    create_task_file(dir.path(), "scratch-env", &[]);
    set_scratch_status_with_instance(
        dir.path(),
        "scratch-env",
        "running",
        json!({
            "branch": "wt/scratch-env",
            "worktree_path": "/tmp/nonexistent-worktree",
            "session_name": "test-session",
            "window_name": "scratch-env"
        }),
    );

    let (ok, _, stderr) = run_wt(dir.path(), &["tail", "scratch-env"]);

    // Scratch can have tail called, but will fail on worktree/transcript
    assert!(!ok);
    // Should fail on worktree, not "task not found"
    assert!(
        stderr.contains("worktree") || stderr.contains("transcript"),
        "Should fail on worktree/transcript, actual error: {}",
        stderr
    );
}
