//! CLI tests for scratch environment behavior
//!
//! Scratch environments (created via `wt new`) have special lifecycle rules:
//! - Cannot use `wt review` or `wt complete`
//! - Can delete directly from Running or Review state
//! - Delete removes entry from status.json entirely (no Completed state)

use crate::common::*;
use serde_json::json;

// ==================== Review Forbidden ====================

#[test]
fn test_scratch_review_forbidden() {
    let dir = setup_test_repo();

    // Create scratch status entry (no task file)
    set_scratch_status(dir.path(), "scratch-env", "running");

    let (ok, _, stderr) = run_wt(dir.path(), &["review", "scratch-env"]);

    assert!(!ok);
    assert!(
        stderr.contains("Scratch") || stderr.contains("cannot be marked"),
        "Expected scratch-specific error, got: {}",
        stderr
    );
}

#[test]
fn test_scratch_review_suggests_delete() {
    let dir = setup_test_repo();

    set_scratch_status(dir.path(), "scratch-env", "running");

    let (ok, _, stderr) = run_wt(dir.path(), &["review", "scratch-env"]);

    assert!(!ok);
    assert!(
        stderr.contains("delete"),
        "Error should suggest using 'wt delete', got: {}",
        stderr
    );
}

// ==================== Complete Forbidden ====================

#[test]
fn test_scratch_complete_forbidden() {
    let dir = setup_test_repo();

    // Create scratch in review state
    set_scratch_status(dir.path(), "scratch-env", "review");

    let (ok, _, stderr) = run_wt(dir.path(), &["complete", "scratch-env"]);

    assert!(!ok);
    assert!(
        stderr.contains("Scratch") || stderr.contains("cannot be completed"),
        "Expected scratch-specific error, got: {}",
        stderr
    );
}

// ==================== Delete Allowed ====================

#[test]
fn test_scratch_delete_allowed_from_running() {
    let dir = setup_test_repo();

    // Create scratch with instance info
    set_scratch_status_with_instance(
        dir.path(),
        "scratch-env",
        "running",
        json!({
            "branch": "wt/scratch-env",
            "worktree_path": "/tmp/nonexistent",
            "session_name": "test-session",
            "window_name": "scratch-env"
        }),
    );

    let (ok, stdout, _) = run_wt(dir.path(), &["delete", "scratch-env"]);

    assert!(ok, "Scratch should be deletable directly from running");
    assert!(
        stdout.contains("deleted") || stdout.contains("Scratch"),
        "Expected scratch delete message, got: {}",
        stdout
    );
}

#[test]
fn test_scratch_delete_removes_from_status() {
    let dir = setup_test_repo();

    set_scratch_status_with_instance(
        dir.path(),
        "scratch-env",
        "running",
        json!({
            "branch": "wt/scratch-env",
            "worktree_path": "/tmp/nonexistent",
            "session_name": "test-session",
            "window_name": "scratch-env"
        }),
    );

    // Verify scratch exists before
    assert!(task_exists_in_status(dir.path(), "scratch-env"));

    let (ok, _, _) = run_wt(dir.path(), &["delete", "scratch-env"]);
    assert!(ok);

    // Verify scratch entry is completely removed
    assert!(
        !task_exists_in_status(dir.path(), "scratch-env"),
        "Scratch should be removed from status.json"
    );
}

// ==================== Reset Behavior ====================

#[test]
fn test_scratch_reset_removes_from_status() {
    let dir = setup_test_repo();

    set_scratch_status_with_instance(
        dir.path(),
        "scratch-env",
        "running",
        json!({
            "branch": "wt/scratch-env",
            "worktree_path": "/tmp/nonexistent",
            "session_name": "test-session",
            "window_name": "scratch-env"
        }),
    );

    // Verify scratch exists before
    assert!(task_exists_in_status(dir.path(), "scratch-env"));

    let (ok, stdout, _) = run_wt(dir.path(), &["reset", "scratch-env"]);
    assert!(ok);

    // Scratch reset should remove entry, not reset to pending
    assert!(
        !task_exists_in_status(dir.path(), "scratch-env"),
        "Scratch should be removed from status.json on reset"
    );

    assert!(
        stdout.contains("cleaned up") || stdout.contains("removed"),
        "Expected cleanup message, got: {}",
        stdout
    );
}

// ==================== Scratch Detection ====================

#[test]
fn test_scratch_identified_by_flag_not_missing_file() {
    let dir = setup_test_repo();

    // Create normal status entry (no scratch flag) without task file
    set_task_status(dir.path(), "orphan", "running");

    // Try to mark for review - should fail because task file not found, not because scratch
    let (ok, _, stderr) = run_wt(dir.path(), &["review", "orphan"]);

    assert!(!ok);
    assert!(
        stderr.contains("not found"),
        "Should fail because task not found, not scratch: {}",
        stderr
    );
}

// ==================== Scratch in List/Status ====================

#[test]
fn test_scratch_not_in_status_without_task_file() {
    let dir = setup_test_repo();

    // Scratch without task file won't appear in status
    // (status command only shows tasks that have task files)
    set_scratch_status_with_instance(
        dir.path(),
        "scratch-env",
        "running",
        json!({
            "branch": "wt/scratch-env",
            "worktree_path": "/tmp/nonexistent",
            "session_name": "test-session",
            "window_name": "scratch-env"
        }),
    );

    let (ok, stdout, _) = run_wt(dir.path(), &["status", "--json"]);
    assert!(ok);

    // Scratch without task file doesn't appear in status
    // This is expected behavior - status shows tasks, not scratch environments
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let tasks = json.get("tasks").and_then(|t| t.as_array()).unwrap();

    // Tasks array should be empty (scratch has no task file)
    assert!(
        tasks.is_empty(),
        "Scratch without task file should not appear in status: {}",
        stdout
    );
}

#[test]
fn test_scratch_not_in_task_list() {
    let dir = setup_test_repo();

    // Create both regular task and scratch
    create_task_file(dir.path(), "regular-task", &[]);
    set_task_status(dir.path(), "regular-task", "pending");
    set_scratch_status(dir.path(), "scratch-env", "running");

    let (ok, stdout, _) = run_wt(dir.path(), &["list", "--json"]);
    assert!(ok);

    // Regular task should appear
    assert!(
        stdout.contains("regular-task"),
        "Regular task should appear in list"
    );

    // Scratch should NOT appear in list (no task file)
    assert!(
        !stdout.contains("scratch-env"),
        "Scratch should not appear in list (no task file)"
    );
}

// ==================== Scratch State Validation ====================

#[test]
fn test_scratch_delete_fails_from_pending() {
    let dir = setup_test_repo();

    // Scratch in pending state (unusual but possible)
    set_scratch_status(dir.path(), "scratch-env", "pending");

    let (ok, _, stderr) = run_wt(dir.path(), &["delete", "scratch-env"]);

    // Should fail - scratch needs to be running or review
    assert!(!ok);
    assert!(
        stderr.contains("Invalid") || stderr.contains("transition"),
        "Expected state error, got: {}",
        stderr
    );
}
