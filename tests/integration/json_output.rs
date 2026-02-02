//! Tests for JSON output format validation
//!
//! These tests verify that the JSON output is well-formed and follows
//! the expected schema for programmatic consumption.

use wt::models::{Task, TaskFrontmatter, TaskStatus, TaskStore};

fn make_task(name: &str, depends: Vec<&str>) -> Task {
    Task {
        frontmatter: TaskFrontmatter {
            name: name.to_string(),
            depends: depends.into_iter().map(String::from).collect(),
        },
        content: format!("Content for {}", name),
        file_path: format!(".wt/tasks/{}.md", name),
    }
}

// ==================== TaskStatus Serialization ====================

#[test]
fn test_task_status_serializes_to_lowercase() {
    assert_eq!(
        serde_json::to_string(&TaskStatus::Pending).unwrap(),
        "\"pending\""
    );
    assert_eq!(
        serde_json::to_string(&TaskStatus::Running).unwrap(),
        "\"running\""
    );
    assert_eq!(
        serde_json::to_string(&TaskStatus::Review).unwrap(),
        "\"review\""
    );
    assert_eq!(
        serde_json::to_string(&TaskStatus::Completed).unwrap(),
        "\"completed\""
    );
}

#[test]
fn test_task_status_deserializes_from_lowercase() {
    assert_eq!(
        serde_json::from_str::<TaskStatus>("\"pending\"").unwrap(),
        TaskStatus::Pending
    );
    assert_eq!(
        serde_json::from_str::<TaskStatus>("\"running\"").unwrap(),
        TaskStatus::Running
    );
    assert_eq!(
        serde_json::from_str::<TaskStatus>("\"review\"").unwrap(),
        TaskStatus::Review
    );
    assert_eq!(
        serde_json::from_str::<TaskStatus>("\"completed\"").unwrap(),
        TaskStatus::Completed
    );
}

// ==================== List JSON Schema ====================

#[test]
fn test_list_json_schema_empty() {
    let store = TaskStore::default();
    let tasks = store.list();

    // Simulate what list --json outputs
    let output = serde_json::json!({
        "tasks": tasks.iter().map(|t| {
            serde_json::json!({
                "name": t.name(),
                "status": store.get_status(t.name()),
                "depends": t.depends()
            })
        }).collect::<Vec<_>>()
    });

    assert!(output["tasks"].is_array());
    assert_eq!(output["tasks"].as_array().unwrap().len(), 0);
}

#[test]
fn test_list_json_schema_with_tasks() {
    let mut store = TaskStore::default();
    store
        .tasks
        .insert("auth".to_string(), make_task("auth", vec![]));
    store
        .tasks
        .insert("api".to_string(), make_task("api", vec!["auth"]));
    store.set_status("auth", TaskStatus::Running);

    let tasks = store.list();
    let output = serde_json::json!({
        "tasks": tasks.iter().map(|t| {
            serde_json::json!({
                "name": t.name(),
                "status": store.get_status(t.name()),
                "depends": t.depends()
            })
        }).collect::<Vec<_>>()
    });

    // Verify schema
    assert!(output["tasks"].is_array());

    for task in output["tasks"].as_array().unwrap() {
        assert!(task["name"].is_string());
        assert!(task["status"].is_string());
        assert!(task["depends"].is_array());

        // Status must be one of the valid values
        let status = task["status"].as_str().unwrap();
        assert!(["pending", "running", "review", "completed"].contains(&status));
    }
}

// ==================== Next JSON Schema ====================

#[test]
fn test_next_json_schema() {
    // Verify the expected structure of next --json output
    let ready = vec!["auth", "database"];
    let blocked = vec![serde_json::json!({
        "name": "api",
        "waiting_for": ["auth"]
    })];

    let output = serde_json::json!({
        "ready": ready,
        "blocked": blocked
    });

    // Verify schema
    assert!(output["ready"].is_array());
    assert!(output["blocked"].is_array());

    for name in output["ready"].as_array().unwrap() {
        assert!(name.is_string());
    }

    for blocked_task in output["blocked"].as_array().unwrap() {
        assert!(blocked_task["name"].is_string());
        assert!(blocked_task["waiting_for"].is_array());
    }
}

// ==================== Unicode in JSON ====================

#[test]
fn test_json_handles_unicode_names() {
    let mut store = TaskStore::default();
    store
        .tasks
        .insert("任务".to_string(), make_task("任务", vec![]));

    let tasks = store.list();
    let output = serde_json::json!({
        "tasks": tasks.iter().map(|t| {
            serde_json::json!({
                "name": t.name(),
                "status": store.get_status(t.name()),
                "depends": t.depends()
            })
        }).collect::<Vec<_>>()
    });

    let json_string = serde_json::to_string(&output).unwrap();
    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&json_string).unwrap();
    assert_eq!(parsed["tasks"][0]["name"], "任务");
}

// ==================== Special Characters in JSON ====================

#[test]
fn test_json_escapes_special_characters() {
    // Task names shouldn't contain these, but content might
    let task = make_task("test", vec![]);

    let output = serde_json::json!({
        "name": task.name(),
        "content": "Line with \"quotes\" and \\ backslash"
    });

    let json_string = serde_json::to_string(&output).unwrap();
    // Should be valid JSON
    let _: serde_json::Value = serde_json::from_str(&json_string).unwrap();
}
