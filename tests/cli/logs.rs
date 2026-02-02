//! CLI tests for wt logs command
//!
//! The logs command extracts filtered transcripts for all tasks.
//! These tests verify basic behavior since actual transcripts require real Claude sessions.

use crate::common::*;
use serde_json::json;

// ==================== Basic Behavior ====================

#[test]
fn test_logs_no_tasks() {
    let dir = setup_test_repo();

    let (ok, stdout, _) = run_wt(dir.path(), &["logs"]);

    assert!(ok);
    assert!(
        stdout.contains("Generated: 0") || stdout.contains("Skipped: 0"),
        "Expected summary line, got: {}",
        stdout
    );
}

#[test]
fn test_logs_skips_pending_tasks() {
    let dir = setup_repo_with_tasks(&[("task1", &[], "pending"), ("task2", &[], "pending")]);

    let (ok, stdout, _) = run_wt(dir.path(), &["logs"]);

    assert!(ok);
    // Pending tasks should be skipped silently
    assert!(
        stdout.contains("Generated: 0"),
        "Pending tasks should be skipped, got: {}",
        stdout
    );
}

#[test]
fn test_logs_processes_running_task_without_transcript() {
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

    let (ok, stdout, _) = run_wt(dir.path(), &["logs"]);

    assert!(ok);
    // Task should be skipped (no transcript found)
    assert!(
        stdout.contains("Skipped: 1") || stdout.contains("Generated: 0"),
        "Task without transcript should be skipped, got: {}",
        stdout
    );
}

#[test]
fn test_logs_processes_done_tasks() {
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

    let (ok, stdout, _) = run_wt(dir.path(), &["logs"]);

    assert!(ok);
    // Done tasks should be processed (but skipped if no transcript)
    assert!(
        stdout.contains("Skipped") || stdout.contains("Generated"),
        "Expected summary output, got: {}",
        stdout
    );
}

// ==================== Task State Filtering ====================

#[test]
fn test_logs_skips_tasks_without_instance() {
    let dir = setup_repo_with_tasks(&[
        ("task1", &[], "running"), // No instance
        ("task2", &[], "review"),  // No instance
    ]);

    let (ok, stdout, _) = run_wt(dir.path(), &["logs"]);

    assert!(ok);
    // Tasks without instance should be skipped
    assert!(
        stdout.contains("Skipped: 2")
            || (stdout.contains("Generated: 0") && stdout.contains("Skipped")),
        "Tasks without instance should be skipped, got: {}",
        stdout
    );
}

#[test]
fn test_logs_ignores_completed_tasks() {
    let dir = setup_repo_with_tasks(&[("task1", &[], "completed")]);

    let (ok, _stdout, _) = run_wt(dir.path(), &["logs"]);

    assert!(ok);
    // Completed tasks should not cause errors
}

#[test]
fn test_logs_ignores_completed_tasks_without_instance() {
    let dir = setup_repo_with_tasks(&[("task1", &[], "completed")]);

    let (ok, stdout, _) = run_wt(dir.path(), &["logs"]);

    assert!(ok);
    // Completed tasks without instance should be skipped gracefully
    assert!(
        stdout.contains("Skipped") || stdout.contains("Generated"),
        "Expected summary output, got: {}",
        stdout
    );
}

// ==================== Summary Output ====================

#[test]
fn test_logs_shows_summary() {
    let dir = setup_test_repo();

    let (ok, stdout, _) = run_wt(dir.path(), &["logs"]);

    assert!(ok);
    assert!(
        stdout.contains("Generated:") && stdout.contains("Skipped:"),
        "Should show summary with Generated and Skipped counts, got: {}",
        stdout
    );
}

#[test]
fn test_logs_multiple_tasks_summary() {
    let dir = setup_repo_with_tasks(&[
        ("task1", &[], "pending"),
        ("task2", &[], "running"),
        ("task3", &[], "review"),
    ]);

    let (ok, stdout, _) = run_wt(dir.path(), &["logs"]);

    assert!(ok);
    // Should process all non-pending tasks
    assert!(stdout.contains("Generated:") && stdout.contains("Skipped:"));
}

// ==================== Mixed Tasks ====================

#[test]
fn test_logs_mixed_states() {
    let dir = setup_test_repo();

    // Create mix of regular tasks and scratch
    create_task_file(dir.path(), "regular", &[]);
    set_task_status(dir.path(), "regular", "pending");

    set_scratch_status(dir.path(), "scratch-env", "running");

    let (ok, stdout, _) = run_wt(dir.path(), &["logs"]);

    assert!(ok);
    // Should handle mix gracefully
    assert!(stdout.contains("Generated:") && stdout.contains("Skipped:"));
}
