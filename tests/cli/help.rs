use super::*;

#[test]
fn test_help() {
    let dir = tempfile::tempdir().unwrap();
    let (ok, stdout, _) = run_wt(dir.path(), &["--help"]);

    assert!(ok);
    assert!(stdout.contains("Worktree Task Manager"));
    assert!(stdout.contains("create"));
    assert!(stdout.contains("list"));
    assert!(stdout.contains("next"));
    assert!(stdout.contains("step"));
}

#[test]
fn test_version() {
    let dir = tempfile::tempdir().unwrap();
    let (ok, stdout, _) = run_wt(dir.path(), &["--version"]);

    assert!(ok);
    assert!(stdout.contains("wt"));
}

#[test]
fn test_list_help_shows_json() {
    let dir = tempfile::tempdir().unwrap();
    let (ok, stdout, _) = run_wt(dir.path(), &["list", "--help"]);

    assert!(ok);
    assert!(stdout.contains("--json"));
}

#[test]
fn test_next_help_shows_task_arg() {
    let dir = tempfile::tempdir().unwrap();
    let (ok, stdout, _) = run_wt(dir.path(), &["next", "--help"]);

    assert!(ok);
    // New next command requires a TASK argument
    assert!(stdout.contains("TASK") || stdout.contains("task"));
}
