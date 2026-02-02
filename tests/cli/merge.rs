use super::*;

#[test]
fn test_merge_nonexistent() {
    let dir = setup_test_repo();
    let (ok, _, stderr) = run_wt(dir.path(), &["merge", "nonexistent"]);

    assert!(!ok);
    assert!(stderr.contains("not found"));
}

#[test]
fn test_merge_pending_task_fails() {
    let dir = setup_repo_with_tasks(&[("task", &[], "pending")]);

    let (ok, _, stderr) = run_wt(dir.path(), &["merge", "task"]);

    // Cannot merge pending task - needs to be done first
    assert!(!ok);
    assert!(stderr.contains("pending") || stderr.contains("expected done"));
}

#[test]
fn test_merge_running_task_fails() {
    let dir = setup_repo_with_tasks(&[("task", &[], "running")]);

    let (ok, _, stderr) = run_wt(dir.path(), &["merge", "task"]);

    // Cannot merge running task - needs to be done first
    assert!(!ok);
    assert!(stderr.contains("running") || stderr.contains("expected done"));
}

#[test]
fn test_merge_merged_task_fails() {
    let dir = setup_repo_with_tasks(&[("task", &[], "merged")]);

    let (ok, _, stderr) = run_wt(dir.path(), &["merge", "task"]);

    // Cannot merge already merged task
    assert!(!ok);
    assert!(stderr.contains("merged") || stderr.contains("expected done"));
}

#[test]
fn test_merge_done_task_without_instance_fails() {
    let dir = setup_repo_with_tasks(&[("task", &[], "done")]);

    let (ok, _, stderr) = run_wt(dir.path(), &["merge", "task"]);

    // Done task without instance (worktree) cannot be merged
    assert!(!ok);
    assert!(stderr.contains("not been started") || stderr.contains("instance"));
}

#[test]
fn test_merge_help() {
    let dir = setup_test_repo();
    let (ok, stdout, _) = run_wt(dir.path(), &["merge", "--help"]);

    assert!(ok);
    assert!(stdout.contains("Execute merge via Claude"));
    assert!(stdout.contains("--agent"));
}
