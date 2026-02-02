use super::*;

#[test]
fn test_list_empty() {
    let dir = setup_test_repo();
    let (ok, stdout, _) = run_wt(dir.path(), &["list"]);

    assert!(ok);
    assert!(stdout.contains("No tasks found"));
}

#[test]
fn test_list_shows_tasks() {
    let dir = setup_test_repo();

    run_wt(dir.path(), &["create", "--json", r#"{"name": "auth", "depends": [], "description": "Auth"}"#]);

    let (ok, stdout, _) = run_wt(dir.path(), &["list"]);

    assert!(ok);
    assert!(stdout.contains("auth"));
}

#[test]
fn test_list_shows_status_symbols() {
    let dir = setup_repo_with_tasks(&[
        ("pending_task", &[], "pending"),
        ("running_task", &[], "running"),
        ("done_task", &[], "done"),
        ("merged_task", &[], "merged"),
    ]);

    let (ok, stdout, _) = run_wt(dir.path(), &["list"]);

    assert!(ok);
    assert!(stdout.contains("○")); // pending
    assert!(stdout.contains("●")); // running
    assert!(stdout.contains("✓")); // done or merged
}

#[test]
fn test_list_shows_grouped_view() {
    let dir = setup_test_repo();

    run_wt(dir.path(), &["create", "--json", r#"{"name": "a", "depends": [], "description": "A"}"#]);
    run_wt(dir.path(), &["create", "--json", r#"{"name": "b", "depends": ["a"], "description": "B"}"#]);

    let (ok, stdout, _) = run_wt(dir.path(), &["list"]);

    assert!(ok);
    // Default view is grouped, should show "Ready" and "Blocked" sections
    assert!(stdout.contains("Ready"));
    assert!(stdout.contains("Blocked"));
    assert!(stdout.contains("← a")); // b is blocked by a (icon has ANSI colors)
}

#[test]
fn test_list_tree_option() {
    let dir = setup_test_repo();

    run_wt(dir.path(), &["create", "--json", r#"{"name": "a", "depends": [], "description": "A"}"#]);
    run_wt(dir.path(), &["create", "--json", r#"{"name": "b", "depends": ["a"], "description": "B"}"#]);

    let (ok, stdout, _) = run_wt(dir.path(), &["list", "--tree"]);

    assert!(ok);
    assert!(stdout.contains("└") || stdout.contains("├"));
}

// ==================== JSON Output ====================

#[test]
fn test_list_json_empty() {
    let dir = setup_test_repo();
    let (ok, stdout, _) = run_wt(dir.path(), &["list", "--json"]);

    assert!(ok);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["tasks"].as_array().unwrap().len(), 0);
}

#[test]
fn test_list_json_single_task() {
    let dir = setup_test_repo();

    run_wt(dir.path(), &["create", "--json", r#"{"name": "auth", "depends": [], "description": "Auth"}"#]);

    let (ok, stdout, _) = run_wt(dir.path(), &["list", "--json"]);

    assert!(ok);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let tasks = json["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["name"], "auth");
    assert_eq!(tasks[0]["status"], "pending");
}

#[test]
fn test_list_json_with_depends() {
    let dir = setup_test_repo();

    run_wt(dir.path(), &["create", "--json", r#"{"name": "a", "depends": [], "description": "A"}"#]);
    run_wt(dir.path(), &["create", "--json", r#"{"name": "b", "depends": ["a"], "description": "B"}"#]);

    let (ok, stdout, _) = run_wt(dir.path(), &["list", "--json"]);

    assert!(ok);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let task_b = json["tasks"].as_array().unwrap().iter().find(|t| t["name"] == "b").unwrap();
    assert_eq!(task_b["depends"].as_array().unwrap(), &[serde_json::json!("a")]);
}

#[test]
fn test_list_json_all_statuses() {
    let dir = setup_repo_with_tasks(&[
        ("p", &[], "pending"),
        ("r", &[], "running"),
        ("d", &[], "done"),
        ("m", &[], "merged"),
    ]);

    let (ok, stdout, _) = run_wt(dir.path(), &["list", "--json"]);

    assert!(ok);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let statuses: Vec<&str> = json["tasks"].as_array().unwrap()
        .iter()
        .map(|t| t["status"].as_str().unwrap())
        .collect();

    assert!(statuses.contains(&"pending"));
    assert!(statuses.contains(&"running"));
    assert!(statuses.contains(&"done"));
    assert!(statuses.contains(&"merged"));
}
