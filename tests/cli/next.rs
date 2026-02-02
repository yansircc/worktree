use super::*;

#[test]
fn test_next_empty() {
    let dir = setup_test_repo();
    let (ok, stdout, _) = run_wt(dir.path(), &["next"]);

    assert!(ok);
    assert!(stdout.contains("No pending tasks"));
}

#[test]
fn test_next_single_ready() {
    let dir = setup_repo_with_tasks(&[("auth", &[], "pending")]);

    let (ok, stdout, _) = run_wt(dir.path(), &["next"]);

    assert!(ok);
    assert!(stdout.contains("Ready to start"));
    assert!(stdout.contains("auth"));
}

#[test]
fn test_next_multiple_ready() {
    let dir = setup_repo_with_tasks(&[
        ("auth", &[], "pending"),
        ("database", &[], "pending"),
    ]);

    let (ok, stdout, _) = run_wt(dir.path(), &["next"]);

    assert!(ok);
    assert!(stdout.contains("auth"));
    assert!(stdout.contains("database"));
}

#[test]
fn test_next_blocked_by_pending() {
    let dir = setup_repo_with_tasks(&[
        ("auth", &[], "pending"),
        ("api", &["auth"], "pending"),
    ]);

    let (ok, stdout, _) = run_wt(dir.path(), &["next"]);

    assert!(ok);
    assert!(stdout.contains("Blocked"));
    assert!(stdout.contains("api"));
    assert!(stdout.contains("waiting for"));
}

#[test]
fn test_next_unblocked_by_merged() {
    let dir = setup_repo_with_tasks(&[
        ("auth", &[], "merged"),
        ("api", &["auth"], "pending"),
    ]);

    let (ok, stdout, _) = run_wt(dir.path(), &["next"]);

    assert!(ok);
    assert!(stdout.contains("Ready"));
    assert!(stdout.contains("api"));
}

#[test]
fn test_next_ignores_non_pending() {
    let dir = setup_repo_with_tasks(&[
        ("running", &[], "running"),
        ("done", &[], "done"),
        ("merged", &[], "merged"),
    ]);

    let (ok, stdout, _) = run_wt(dir.path(), &["next"]);

    assert!(ok);
    assert!(stdout.contains("No pending tasks"));
}

#[test]
fn test_next_diamond_dependency() {
    let dir = setup_repo_with_tasks(&[
        ("a", &[], "merged"),
        ("b", &["a"], "pending"),
        ("c", &["a"], "pending"),
        ("d", &["b", "c"], "pending"),
    ]);

    let (ok, stdout, _) = run_wt(dir.path(), &["next", "--json"]);

    assert!(ok);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let ready: Vec<&str> = json["ready"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["name"].as_str().unwrap())
        .collect();

    assert!(ready.contains(&"b"));
    assert!(ready.contains(&"c"));
    assert!(!ready.contains(&"d"));
}

// ==================== JSON Output ====================

#[test]
fn test_next_json_empty() {
    let dir = setup_test_repo();
    let (ok, stdout, _) = run_wt(dir.path(), &["next", "--json"]);

    assert!(ok);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json["ready"].as_array().unwrap().is_empty());
    assert!(json["blocked"].as_array().unwrap().is_empty());
}

#[test]
fn test_next_json_ready() {
    let dir = setup_repo_with_tasks(&[("auth", &[], "pending")]);

    let (ok, stdout, _) = run_wt(dir.path(), &["next", "--json"]);

    assert!(ok);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["ready"][0]["name"], "auth");
    assert_eq!(json["ready"][0]["index"], 1);
}

#[test]
fn test_next_json_blocked() {
    let dir = setup_repo_with_tasks(&[
        ("auth", &[], "pending"),
        ("api", &["auth"], "pending"),
    ]);

    let (ok, stdout, _) = run_wt(dir.path(), &["next", "--json"]);

    assert!(ok);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["blocked"][0]["name"], "api");
    assert!(json["blocked"][0]["waiting_for"].as_array().unwrap().contains(&serde_json::json!("auth")));
}

#[test]
fn test_next_json_multiple_blockers() {
    let dir = setup_repo_with_tasks(&[
        ("a", &[], "pending"),
        ("b", &[], "pending"),
        ("c", &["a", "b"], "pending"),
    ]);

    let (ok, stdout, _) = run_wt(dir.path(), &["next", "--json"]);

    assert!(ok);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let waiting_for = json["blocked"][0]["waiting_for"].as_array().unwrap();
    assert_eq!(waiting_for.len(), 2);
}
