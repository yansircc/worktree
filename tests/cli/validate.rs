use super::*;

#[test]
fn test_validate_empty() {
    let dir = setup_test_repo();
    let (ok, stdout, _) = run_wt(dir.path(), &["validate"]);

    assert!(ok);
    assert!(stdout.contains("No tasks found"));
}

#[test]
fn test_validate_all_valid() {
    let dir = setup_test_repo();

    run_wt(
        dir.path(),
        &[
            "create",
            "--json",
            r#"{"name": "a", "depends": [], "description": "A"}"#,
        ],
    );
    run_wt(
        dir.path(),
        &[
            "create",
            "--json",
            r#"{"name": "b", "depends": ["a"], "description": "B"}"#,
        ],
    );

    let (ok, stdout, _) = run_wt(dir.path(), &["validate"]);

    assert!(ok);
    assert!(stdout.contains("valid"));
}

#[test]
fn test_validate_specific_task() {
    let dir = setup_test_repo();

    run_wt(
        dir.path(),
        &[
            "create",
            "--json",
            r#"{"name": "task1", "depends": [], "description": "Test"}"#,
        ],
    );

    let (ok, stdout, _) = run_wt(dir.path(), &["validate", "task1"]);

    assert!(ok);
    assert!(stdout.contains("valid"));
}

#[test]
fn test_validate_nonexistent_task() {
    let dir = setup_test_repo();
    let (ok, _, stderr) = run_wt(dir.path(), &["validate", "nonexistent"]);

    assert!(!ok);
    assert!(stderr.contains("not found"));
}

#[test]
fn test_validate_detects_cycle() {
    let dir = setup_test_repo();

    fs::create_dir_all(dir.path().join(".wt/tasks")).unwrap();
    fs::write(
        dir.path().join(".wt/tasks/a.md"),
        "---\nname: a\nstatus: pending\ndepends:\n  - b\n---\n\nA",
    )
    .unwrap();
    fs::write(
        dir.path().join(".wt/tasks/b.md"),
        "---\nname: b\nstatus: pending\ndepends:\n  - a\n---\n\nB",
    )
    .unwrap();

    let (_, stdout, _) = run_wt(dir.path(), &["validate"]);

    assert!(stdout.contains("circular") || stdout.contains("error"));
}
