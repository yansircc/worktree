//! CLI tests for wt status command

use crate::common::*;
use serde_json::json;

#[test]
fn test_status_no_tasks() {
    let dir = setup_test_repo();

    // Non-TTY environment auto-degrades to JSON
    let (ok, stdout, _stderr) = run_wt(dir.path(), &["status", "--json"]);

    assert!(ok);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let tasks = json.get("tasks").unwrap().as_array().unwrap();
    assert!(tasks.is_empty());
}

#[test]
fn test_status_with_running_task() {
    let dir = setup_repo_with_tasks(&[("task1", &[], "active")]);

    let (ok, stdout, _stderr) = run_wt(dir.path(), &["status", "--json"]);

    assert!(ok);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let tasks = json.get("tasks").unwrap().as_array().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].get("name").unwrap().as_str().unwrap(), "task1");
}

#[test]
fn test_status_with_review_task() {
    let dir = setup_repo_with_tasks(&[("task1", &[], "idle")]);

    let (ok, stdout, _stderr) = run_wt(dir.path(), &["status", "--json"]);

    assert!(ok);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let tasks = json.get("tasks").unwrap().as_array().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].get("status").unwrap().as_str().unwrap(), "idle");
}

#[test]
fn test_status_ignores_pending_tasks() {
    let dir = setup_repo_with_tasks(&[("task1", &[], "pending"), ("task2", &[], "active")]);

    let (ok, stdout, _stderr) = run_wt(dir.path(), &["status", "--json"]);

    assert!(ok);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let tasks = json.get("tasks").unwrap().as_array().unwrap();
    // Only task2 should be shown (running), task1 (pending) is ignored
    // Note: task2 will be auto-marked as review since tmux window doesn't exist
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].get("name").unwrap().as_str().unwrap(), "task2");
}

#[test]
fn test_status_ignores_completed_tasks() {
    let dir = setup_repo_with_tasks(&[("task1", &[], "completed"), ("task2", &[], "active")]);

    let (ok, stdout, _stderr) = run_wt(dir.path(), &["status", "--json"]);

    assert!(ok);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let tasks = json.get("tasks").unwrap().as_array().unwrap();
    // Only task2 should be shown, task1 (completed) is ignored
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].get("name").unwrap().as_str().unwrap(), "task2");
}

#[test]
fn test_status_json_output() {
    let dir = setup_repo_with_tasks(&[("task1", &[], "active")]);

    let (ok, stdout, _stderr) = run_wt(dir.path(), &["status", "--json"]);

    assert!(ok);
    // Should be valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(parsed.is_ok(), "Output should be valid JSON: {}", stdout);

    let json = parsed.unwrap();
    assert!(json.get("tasks").is_some());
    assert!(json.get("summary").is_some());
}

#[test]
fn test_status_json_structure() {
    // Note: "active" tasks without a real tmux window get auto-marked as "idle"
    let dir = setup_repo_with_tasks(&[("task1", &[], "idle"), ("task2", &[], "idle")]);

    let (ok, stdout, _stderr) = run_wt(dir.path(), &["status", "--json"]);

    assert!(ok);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Check tasks array
    let tasks = json.get("tasks").unwrap().as_array().unwrap();
    assert_eq!(tasks.len(), 2);

    // Check summary
    let summary = json.get("summary").unwrap();
    assert_eq!(summary.get("active").unwrap().as_i64().unwrap(), 0);
    assert_eq!(summary.get("idle").unwrap().as_i64().unwrap(), 2);
}

#[test]
fn test_status_summary_line() {
    // Note: "active" tasks without a real tmux window get auto-marked as "idle"
    let dir = setup_repo_with_tasks(&[("task1", &[], "idle"), ("task2", &[], "idle")]);

    let (ok, stdout, _stderr) = run_wt(dir.path(), &["status", "--json"]);

    assert!(ok);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let summary = json.get("summary").unwrap();
    assert_eq!(summary.get("active").unwrap().as_i64().unwrap(), 0);
    assert_eq!(summary.get("idle").unwrap().as_i64().unwrap(), 2);
}

#[test]
fn test_status_auto_marks_review_when_tmux_window_closed() {
    let dir = setup_test_repo();

    // Create task file
    create_task_file(dir.path(), "task1", &[]);

    // Set running status with instance pointing to non-existent tmux window
    set_task_status_with_instance(
        dir.path(),
        "task1",
        "active",
        Some(json!({
            "branch": "wt/task1",
            "worktree_path": "/tmp/nonexistent",
            "session_name": "nonexistent-session-12345",
            "window_name": "task1"
        })),
    );

    let (ok, stdout, _stderr) = run_wt(dir.path(), &["status", "--json"]);

    assert!(ok);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let tasks = json.get("tasks").unwrap().as_array().unwrap();
    let task = &tasks[0];
    // Task should be auto-marked as review when tmux window is closed
    assert_eq!(task.get("status").unwrap().as_str().unwrap(), "idle");
}

#[test]
fn test_status_json_auto_marks_review_when_tmux_closed() {
    let dir = setup_test_repo();

    // Create task file
    create_task_file(dir.path(), "task1", &[]);

    // Set running status with instance pointing to non-existent tmux window
    set_task_status_with_instance(
        dir.path(),
        "task1",
        "active",
        Some(json!({
            "branch": "wt/task1",
            "worktree_path": "/tmp/nonexistent",
            "session_name": "nonexistent-session-12345",
            "window_name": "task1"
        })),
    );

    let (ok, stdout, _stderr) = run_wt(dir.path(), &["status", "--json"]);

    assert!(ok);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let tasks = json.get("tasks").unwrap().as_array().unwrap();
    let task = &tasks[0];

    // Task should be auto-marked as review, tmux_alive is not included for review tasks
    assert_eq!(task.get("status").unwrap().as_str().unwrap(), "idle");
    assert!(
        task.get("tmux_alive").is_none(),
        "tmux_alive should not be included for review tasks"
    );
}
