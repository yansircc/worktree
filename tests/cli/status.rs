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
    let dir = setup_repo_with_tasks(&[("task1", &[], "running")]);

    let (ok, stdout, _stderr) = run_wt(dir.path(), &["status", "--json"]);

    assert!(ok);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let tasks = json.get("tasks").unwrap().as_array().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].get("name").unwrap().as_str().unwrap(), "task1");
}

#[test]
fn test_status_with_done_task() {
    let dir = setup_repo_with_tasks(&[("task1", &[], "done")]);

    let (ok, stdout, _stderr) = run_wt(dir.path(), &["status", "--json"]);

    assert!(ok);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let tasks = json.get("tasks").unwrap().as_array().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].get("status").unwrap().as_str().unwrap(), "done");
}

#[test]
fn test_status_ignores_pending_tasks() {
    let dir = setup_repo_with_tasks(&[
        ("task1", &[], "pending"),
        ("task2", &[], "running"),
    ]);

    let (ok, stdout, _stderr) = run_wt(dir.path(), &["status", "--json"]);

    assert!(ok);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let tasks = json.get("tasks").unwrap().as_array().unwrap();
    // Only task2 should be shown (running), task1 (pending) is ignored
    // Note: task2 will be auto-marked as done since tmux window doesn't exist
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].get("name").unwrap().as_str().unwrap(), "task2");
}

#[test]
fn test_status_ignores_merged_tasks() {
    let dir = setup_repo_with_tasks(&[
        ("task1", &[], "merged"),
        ("task2", &[], "running"),
    ]);

    let (ok, stdout, _stderr) = run_wt(dir.path(), &["status", "--json"]);

    assert!(ok);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let tasks = json.get("tasks").unwrap().as_array().unwrap();
    // Only task2 should be shown, task1 (merged) is ignored
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].get("name").unwrap().as_str().unwrap(), "task2");
}

#[test]
fn test_status_json_output() {
    let dir = setup_repo_with_tasks(&[("task1", &[], "running")]);

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
    // Note: "running" tasks without a real tmux window get auto-marked as "done"
    let dir = setup_repo_with_tasks(&[
        ("task1", &[], "done"),
        ("task2", &[], "done"),
    ]);

    let (ok, stdout, _stderr) = run_wt(dir.path(), &["status", "--json"]);

    assert!(ok);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Check tasks array
    let tasks = json.get("tasks").unwrap().as_array().unwrap();
    assert_eq!(tasks.len(), 2);

    // Check summary
    let summary = json.get("summary").unwrap();
    assert_eq!(summary.get("running").unwrap().as_i64().unwrap(), 0);
    assert_eq!(summary.get("done").unwrap().as_i64().unwrap(), 2);
}

#[test]
fn test_status_summary_line() {
    // Note: "running" tasks without a real tmux window get auto-marked as "done"
    let dir = setup_repo_with_tasks(&[
        ("task1", &[], "done"),
        ("task2", &[], "done"),
    ]);

    let (ok, stdout, _stderr) = run_wt(dir.path(), &["status", "--json"]);

    assert!(ok);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let summary = json.get("summary").unwrap();
    assert_eq!(summary.get("running").unwrap().as_i64().unwrap(), 0);
    assert_eq!(summary.get("done").unwrap().as_i64().unwrap(), 2);
}

#[test]
fn test_status_auto_marks_done_when_tmux_window_closed() {
    let dir = setup_test_repo();

    // Create task file
    create_task_file(dir.path(), "task1", &[]);

    // Set running status with instance pointing to non-existent tmux window
    set_task_status_with_instance(
        dir.path(),
        "task1",
        "running",
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
    // Task should be auto-marked as done when tmux window is closed
    assert_eq!(task.get("status").unwrap().as_str().unwrap(), "done");
}

#[test]
fn test_status_json_auto_marks_done_when_tmux_closed() {
    let dir = setup_test_repo();

    // Create task file
    create_task_file(dir.path(), "task1", &[]);

    // Set running status with instance pointing to non-existent tmux window
    set_task_status_with_instance(
        dir.path(),
        "task1",
        "running",
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

    // Task should be auto-marked as done, tmux_alive is not included for done tasks
    assert_eq!(task.get("status").unwrap().as_str().unwrap(), "done");
    assert!(task.get("tmux_alive").is_none(), "tmux_alive should not be included for done tasks");
}
