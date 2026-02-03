//! Tests for `wt next <task>` command (Phases v2)
//!
//! The next command advances a task to the next phase:
//! pending -> developing -> reviewing -> merging -> completed

use super::*;

#[test]
fn test_next_requires_task_arg() {
    let dir = setup_test_repo();
    let (ok, _, stderr) = run_wt(dir.path(), &["next"]);

    assert!(!ok);
    assert!(stderr.contains("required") || stderr.contains("TASK"));
}

#[test]
fn test_next_task_not_found() {
    let dir = setup_test_repo();
    let (ok, _, stderr) = run_wt(dir.path(), &["next", "nonexistent"]);

    assert!(!ok);
    assert!(stderr.contains("not found"));
}

#[test]
fn test_next_pending_to_developing() {
    let dir = setup_repo_with_tasks(&[("auth", &[], "pending")]);

    let (ok, stdout, _) = run_wt(dir.path(), &["next", "auth"]);

    assert!(ok);
    assert!(stdout.contains("developing") || stdout.contains("advanced"));
}

#[test]
fn test_next_by_index() {
    let dir = setup_repo_with_tasks(&[("auth", &[], "pending")]);

    // Task "auth" should be index 1
    let (ok, stdout, _) = run_wt(dir.path(), &["next", "1"]);

    assert!(ok);
    assert!(stdout.contains("auth") || stdout.contains("developing"));
}

#[test]
fn test_next_already_completed_error() {
    let dir = setup_repo_with_tasks(&[("auth", &[], "completed")]);

    let (ok, _, stderr) = run_wt(dir.path(), &["next", "auth"]);

    assert!(!ok);
    assert!(stderr.contains("completed") || stderr.contains("already"));
}

#[test]
fn test_next_updates_status_file() {
    let dir = setup_repo_with_tasks(&[("auth", &[], "pending")]);

    // Run next to advance to developing
    let (ok, _, _) = run_wt(dir.path(), &["next", "auth"]);
    assert!(ok);

    // Check status via list command
    let (ok, stdout, _) = run_wt(dir.path(), &["list", "--json"]);
    assert!(ok);

    // Parse JSON and check phase
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let tasks = json["tasks"].as_array().unwrap();
    let auth = tasks.iter().find(|t| t["name"] == "auth").unwrap();

    // After next, should be in developing phase and idle status
    // (idle because we didn't start the workflow)
    assert!(
        auth["phase"].as_str().map(|s| s.contains("developing")).unwrap_or(false) ||
        auth["status"].as_str().map(|s| s != "pending").unwrap_or(false),
        "Task should have advanced from pending. Got: {:?}", auth
    );
}
