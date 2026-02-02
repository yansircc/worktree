use super::*;

#[test]
fn test_resume_nonexistent() {
    let dir = setup_test_repo();
    let (ok, _, stderr) = run_wt(dir.path(), &["resume", "nonexistent"]);

    assert!(!ok);
    assert!(stderr.contains("not found"));
}

#[test]
fn test_resume_pending_task() {
    let dir = setup_repo_with_tasks(&[("task", &[], "pending")]);

    let (ok, _, stderr) = run_wt(dir.path(), &["resume", "task"]);

    assert!(!ok);
    assert!(stderr.contains("idle") || stderr.contains("expected review"));
}

#[test]
fn test_resume_running_task() {
    let dir = setup_repo_with_tasks(&[("task", &[], "active")]);

    let (ok, _, stderr) = run_wt(dir.path(), &["resume", "task"]);

    assert!(!ok);
    assert!(stderr.contains("idle") || stderr.contains("expected review"));
}

#[test]
fn test_resume_review_task_without_instance() {
    let dir = setup_repo_with_tasks(&[("task", &[], "idle")]);

    let (ok, _, stderr) = run_wt(dir.path(), &["resume", "task"]);

    assert!(!ok);
    // Should fail because no instance exists
    assert!(stderr.contains("not been started") || stderr.contains("instance"));
}
