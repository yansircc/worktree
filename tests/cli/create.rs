use super::*;

#[test]
fn test_create_simple_task() {
    let dir = setup_test_repo();
    let (ok, stdout, _) = run_wt(
        dir.path(),
        &[
            "create",
            "--json",
            r#"{"name": "auth", "depends": [], "description": "Implement auth"}"#,
        ],
    );

    assert!(ok);
    assert!(stdout.contains("Task 'auth' created"));
    assert!(dir.path().join(".wt/tasks/auth.md").exists());
}

#[test]
fn test_create_with_depends() {
    let dir = setup_test_repo();

    run_wt(
        dir.path(),
        &[
            "create",
            "--json",
            r#"{"name": "auth", "depends": [], "description": "Auth"}"#,
        ],
    );

    let (ok, stdout, _) = run_wt(
        dir.path(),
        &[
            "create",
            "--json",
            r#"{"name": "api", "depends": ["auth"], "description": "API"}"#,
        ],
    );

    assert!(ok);
    assert!(stdout.contains("Task 'api' created"));
    assert!(stdout.contains("Depends: auth"));
}

#[test]
fn test_create_duplicate_task() {
    let dir = setup_test_repo();

    run_wt(
        dir.path(),
        &[
            "create",
            "--json",
            r#"{"name": "task1", "depends": [], "description": "Test"}"#,
        ],
    );

    let (ok, _, stderr) = run_wt(
        dir.path(),
        &[
            "create",
            "--json",
            r#"{"name": "task1", "depends": [], "description": "Test"}"#,
        ],
    );

    assert!(!ok);
    assert!(stderr.contains("already exists"));
}

#[test]
fn test_create_invalid_json() {
    let dir = setup_test_repo();
    let (ok, _, stderr) = run_wt(dir.path(), &["create", "--json", "not json"]);

    assert!(!ok);
    assert!(stderr.contains("Invalid JSON"));
}

#[test]
fn test_create_missing_description() {
    let dir = setup_test_repo();
    let (ok, _, stderr) = run_wt(
        dir.path(),
        &["create", "--json", r#"{"name": "test", "depends": []}"#],
    );

    assert!(!ok);
    assert!(stderr.contains("description") || stderr.contains("missing field"));
}

#[test]
fn test_create_empty_description() {
    let dir = setup_test_repo();
    let (ok, stdout, _) = run_wt(
        dir.path(),
        &[
            "create",
            "--json",
            r#"{"name": "test", "depends": [], "description": ""}"#,
        ],
    );

    // Empty description is allowed
    assert!(ok);
    assert!(stdout.contains("created"));
}

#[test]
fn test_create_missing_dependency() {
    let dir = setup_test_repo();
    let (ok, _, stderr) = run_wt(
        dir.path(),
        &[
            "create",
            "--json",
            r#"{"name": "test", "depends": ["nonexistent"], "description": "Test"}"#,
        ],
    );

    assert!(!ok);
    assert!(stderr.contains("not found") || stderr.contains("nonexistent"));
}

#[test]
fn test_create_invalid_name_with_space() {
    let dir = setup_test_repo();
    let (ok, _, stderr) = run_wt(
        dir.path(),
        &[
            "create",
            "--json",
            r#"{"name": "has space", "depends": [], "description": "Test"}"#,
        ],
    );

    assert!(!ok);
    assert!(stderr.contains("whitespace"));
}

#[test]
fn test_create_invalid_name_with_special_chars() {
    let dir = setup_test_repo();

    for char in ["~", "^", ":", "?", "*", "@"] {
        let json = format!(
            r#"{{"name": "test{}name", "depends": [], "description": "Test"}}"#,
            char
        );
        let (ok, _, stderr) = run_wt(dir.path(), &["create", "--json", &json]);

        assert!(!ok, "Should reject name with {}", char);
        assert!(stderr.contains("invalid") || stderr.contains(char));
    }
}

#[test]
fn test_create_invalid_name_start_with_dash() {
    let dir = setup_test_repo();
    let (ok, _, stderr) = run_wt(
        dir.path(),
        &[
            "create",
            "--json",
            r#"{"name": "-task", "depends": [], "description": "Test"}"#,
        ],
    );

    assert!(!ok);
    assert!(stderr.contains("start"));
}

#[test]
fn test_create_invalid_name_end_with_lock() {
    let dir = setup_test_repo();
    let (ok, _, stderr) = run_wt(
        dir.path(),
        &[
            "create",
            "--json",
            r#"{"name": "task.lock", "depends": [], "description": "Test"}"#,
        ],
    );

    assert!(!ok);
    assert!(stderr.contains(".lock"));
}
