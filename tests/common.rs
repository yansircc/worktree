//! Shared test utilities
//!
//! This module provides common helpers for CLI and integration tests.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Run wt command and return (success, stdout, stderr)
pub fn run_wt(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_wt"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("Failed to execute wt");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

/// Setup a minimal test git repo with wt config
pub fn setup_test_repo() -> TempDir {
    let dir = tempfile::tempdir().unwrap();

    Command::new("git")
        .current_dir(dir.path())
        .args(["init"])
        .output()
        .expect("Failed to init git");

    Command::new("git")
        .current_dir(dir.path())
        .args(["config", "user.email", "test@test.com"])
        .output()
        .ok();
    Command::new("git")
        .current_dir(dir.path())
        .args(["config", "user.name", "Test"])
        .output()
        .ok();

    fs::write(dir.path().join("README.md"), "# Test").unwrap();

    Command::new("git")
        .current_dir(dir.path())
        .args(["add", "."])
        .output()
        .ok();
    Command::new("git")
        .current_dir(dir.path())
        .args(["commit", "-m", "init"])
        .output()
        .ok();

    fs::create_dir_all(dir.path().join(".wt")).unwrap();
    fs::write(
        dir.path().join(".wt/config.jsonc"),
        r#"{"multiplexer": "tmux", "session_name": "test-wt"}"#,
    )
    .unwrap();

    dir
}

/// Setup repo with pre-created tasks: (name, depends, status)
pub fn setup_repo_with_tasks(tasks: &[(&str, &[&str], &str)]) -> TempDir {
    let dir = setup_test_repo();
    for (name, depends, status) in tasks {
        create_task_file(dir.path(), name, depends);
        set_task_status(dir.path(), name, status);
    }
    dir
}

/// Create a task file directly (bypasses validation)
/// Note: status is now stored separately in status.json
pub fn create_task_file(dir: &Path, name: &str, depends: &[&str]) {
    let tasks_dir = dir.join(".wt/tasks");
    fs::create_dir_all(&tasks_dir).unwrap();

    let depends_yaml = if depends.is_empty() {
        String::new()
    } else {
        format!(
            "depends:\n{}\n",
            depends
                .iter()
                .map(|d| format!("  - {}", d))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    let content = format!("---\nname: {}\n{}---\n\nTask {}", name, depends_yaml, name);
    fs::write(tasks_dir.join(format!("{}.md", name)), content).unwrap();
}

/// Set task status in status.json
pub fn set_task_status(dir: &Path, name: &str, status: &str) {
    set_task_status_with_instance(dir, name, status, None);
}

/// Set task status with optional instance info in status.json
pub fn set_task_status_with_instance(
    dir: &Path,
    name: &str,
    status: &str,
    instance: Option<serde_json::Value>,
) {
    let status_file = dir.join(".wt/status.json");
    let wt_dir = dir.join(".wt");
    fs::create_dir_all(&wt_dir).unwrap();

    // Load existing status or create new
    let mut status_data: HashMap<String, serde_json::Value> = if status_file.exists() {
        let content = fs::read_to_string(&status_file).unwrap();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    };

    // Get or create tasks map
    let tasks = status_data
        .entry("tasks".to_string())
        .or_insert_with(|| serde_json::json!({}));

    // Set task status with optional instance
    if let Some(tasks_obj) = tasks.as_object_mut() {
        let mut task_data = serde_json::json!({ "status": status });
        if let Some(inst) = instance {
            task_data["instance"] = inst;
        }
        tasks_obj.insert(name.to_string(), task_data);
    }

    fs::write(
        &status_file,
        serde_json::to_string_pretty(&status_data).unwrap(),
    )
    .unwrap();
}

/// Set scratch status for a task (scratch=true in status.json)
pub fn set_scratch_status(dir: &Path, name: &str, status: &str) {
    let status_file = dir.join(".wt/status.json");
    let wt_dir = dir.join(".wt");
    fs::create_dir_all(&wt_dir).unwrap();

    // Load existing status or create new
    let mut status_data: HashMap<String, serde_json::Value> = if status_file.exists() {
        let content = fs::read_to_string(&status_file).unwrap();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    };

    // Get or create tasks map
    let tasks = status_data
        .entry("tasks".to_string())
        .or_insert_with(|| serde_json::json!({}));

    // Set task status with scratch=true
    if let Some(tasks_obj) = tasks.as_object_mut() {
        let task_data = serde_json::json!({
            "status": status,
            "scratch": true
        });
        tasks_obj.insert(name.to_string(), task_data);
    }

    fs::write(
        &status_file,
        serde_json::to_string_pretty(&status_data).unwrap(),
    )
    .unwrap();
}

/// Set scratch status with instance info
pub fn set_scratch_status_with_instance(
    dir: &Path,
    name: &str,
    status: &str,
    instance: serde_json::Value,
) {
    let status_file = dir.join(".wt/status.json");
    let wt_dir = dir.join(".wt");
    fs::create_dir_all(&wt_dir).unwrap();

    let mut status_data: HashMap<String, serde_json::Value> = if status_file.exists() {
        let content = fs::read_to_string(&status_file).unwrap();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    };

    let tasks = status_data
        .entry("tasks".to_string())
        .or_insert_with(|| serde_json::json!({}));

    if let Some(tasks_obj) = tasks.as_object_mut() {
        let task_data = serde_json::json!({
            "status": status,
            "scratch": true,
            "instance": instance
        });
        tasks_obj.insert(name.to_string(), task_data);
    }

    fs::write(
        &status_file,
        serde_json::to_string_pretty(&status_data).unwrap(),
    )
    .unwrap();
}

/// Parse status.json and return the Value
pub fn parse_status_json(dir: &Path) -> serde_json::Value {
    let status_file = dir.join(".wt/status.json");
    if status_file.exists() {
        let content = fs::read_to_string(&status_file).unwrap();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        serde_json::json!({ "tasks": {} })
    }
}

/// Get a specific task from status.json
pub fn get_task_from_status(dir: &Path, name: &str) -> Option<serde_json::Value> {
    let status = parse_status_json(dir);
    status.get("tasks").and_then(|t| t.get(name)).cloned()
}

/// Check if task exists in status.json
pub fn task_exists_in_status(dir: &Path, name: &str) -> bool {
    get_task_from_status(dir, name).is_some()
}
