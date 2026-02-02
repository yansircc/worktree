use super::*;

#[test]
fn test_review_nonexistent() {
    let dir = setup_test_repo();
    let (ok, _, stderr) = run_wt(dir.path(), &["review", "nonexistent"]);

    assert!(!ok);
    assert!(stderr.contains("not found"));
}

#[test]
fn test_review_pending_task() {
    let dir = setup_repo_with_tasks(&[("task", &[], "pending")]);

    let (ok, _, stderr) = run_wt(dir.path(), &["review", "task"]);

    assert!(!ok);
    assert!(
        stderr.contains("no running")
            || stderr.contains("instance")
            || stderr.contains("Invalid state")
    );
}

#[test]
fn test_review_already_review() {
    let dir = setup_repo_with_tasks(&[("task", &[], "idle")]);

    let (ok, _, stderr) = run_wt(dir.path(), &["review", "task"]);

    assert!(!ok);
    assert!(
        stderr.contains("no running")
            || stderr.contains("instance")
            || stderr.contains("Invalid state")
    );
}
