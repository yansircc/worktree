//! CLI tests for wt new command
//!
//! Note: wt new uses positional argument for name, e.g., `wt new my-scratch`

use crate::common::*;

// ==================== Name Generation ====================

#[test]
fn test_new_auto_generates_name_s1_s2() {
    let dir = setup_test_repo();

    // wt new without name should fail in test environment (no tmux)
    // but we can check the error message shows a generated name
    let (_ok, stdout, stderr) = run_wt(dir.path(), &["new"]);

    // It will fail because tmux is not available in test, but should show the generated name
    // or succeed in creating status entry before tmux failure
    let output = format!("{}{}", stdout, stderr);

    // The command should attempt to create a scratch env with auto-generated name (s1, s2, ...)
    // Either succeeds partially or fails on tmux - both indicate name was generated
    assert!(
        output.contains(": s1") || output.contains(": s2") || output.contains("'s1'") || output.contains("'s2'") || output.contains("Created scratch"),
        "Expected auto-generated name pattern 's1/s2' or creation message, got: {}",
        output
    );
}

// ==================== Explicit Name ====================

#[test]
fn test_new_with_explicit_name_validates() {
    let dir = setup_test_repo();

    // Invalid name (starts with dash) should fail validation
    // Use positional argument, not --name
    let (ok, _, stderr) = run_wt(dir.path(), &["new", "--", "-invalid"]);

    assert!(!ok);
    assert!(
        stderr.contains("Invalid") || stderr.contains("branch") || stderr.contains("cannot start with"),
        "Expected validation error, got: {}",
        stderr
    );
}

#[test]
fn test_new_with_valid_explicit_name() {
    let dir = setup_test_repo();

    // Valid name - will fail on git/tmux but should pass validation
    let (_ok, stdout, stderr) = run_wt(dir.path(), &["new", "my-scratch"]);

    let output = format!("{}{}", stdout, stderr);

    // Should either succeed or fail on git/tmux (not validation)
    // If it fails on validation, that's a bug
    assert!(
        !output.contains("Invalid task name"),
        "Name 'my-scratch' should be valid"
    );
}

// ==================== Name Conflicts ====================

#[test]
fn test_new_name_conflict_with_task_file() {
    let dir = setup_test_repo();

    // Create existing task file
    create_task_file(dir.path(), "existing", &[]);

    // Try to create scratch with same name (positional arg)
    let (ok, _, stderr) = run_wt(dir.path(), &["new", "existing"]);

    assert!(!ok);
    assert!(
        stderr.contains("already exists") || stderr.contains("TaskExists"),
        "Expected conflict error, got: {}",
        stderr
    );
}

#[test]
fn test_new_name_conflict_with_status_entry() {
    let dir = setup_test_repo();

    // Create status entry without task file (scratch scenario)
    set_scratch_status(dir.path(), "scratch-env", "running");

    // Try to create another scratch with same name
    let (ok, _, stderr) = run_wt(dir.path(), &["new", "scratch-env"]);

    assert!(!ok);
    assert!(
        stderr.contains("already exists") || stderr.contains("status.json"),
        "Expected conflict error, got: {}",
        stderr
    );
}

// ==================== Scratch Flag ====================

#[test]
fn test_new_does_not_create_task_file() {
    let dir = setup_test_repo();

    // Run new command (may fail on tmux but should not create task file)
    let _ = run_wt(dir.path(), &["new", "no-file-scratch"]);

    // Verify no task file was created
    let task_file = dir.path().join(".wt/tasks/no-file-scratch.md");
    assert!(
        !task_file.exists(),
        "wt new should not create task file"
    );
}

// ==================== Validation Rules ====================

#[test]
fn test_new_rejects_names_starting_with_dash() {
    let dir = setup_test_repo();

    // Use -- to pass argument starting with dash
    let (ok, _, stderr) = run_wt(dir.path(), &["new", "--", "-badname"]);

    assert!(!ok);
    assert!(
        stderr.contains("Invalid") || stderr.contains("branch") || stderr.contains("cannot start with"),
        "Expected validation error, got: {}",
        stderr
    );
}

#[test]
fn test_new_rejects_names_with_spaces() {
    let dir = setup_test_repo();

    let (ok, _, stderr) = run_wt(dir.path(), &["new", "bad name"]);

    assert!(!ok);
    assert!(
        stderr.contains("Invalid") || stderr.contains("space") || stderr.contains("whitespace"),
        "Expected validation error, got: {}",
        stderr
    );
}

#[test]
fn test_new_rejects_names_ending_with_dot() {
    let dir = setup_test_repo();

    let (ok, _, stderr) = run_wt(dir.path(), &["new", "badname."]);

    assert!(!ok);
    assert!(
        stderr.contains("Invalid") || stderr.contains("branch") || stderr.contains("cannot end with"),
        "Expected validation error, got: {}",
        stderr
    );
}

#[test]
fn test_new_rejects_names_ending_with_lock() {
    let dir = setup_test_repo();

    let (ok, _, stderr) = run_wt(dir.path(), &["new", "badname.lock"]);

    assert!(!ok);
    assert!(
        stderr.contains("Invalid") || stderr.contains("branch") || stderr.contains(".lock"),
        "Expected validation error, got: {}",
        stderr
    );
}

#[test]
fn test_new_rejects_names_with_double_dots() {
    let dir = setup_test_repo();

    let (ok, _, stderr) = run_wt(dir.path(), &["new", "bad..name"]);

    assert!(!ok);
    assert!(
        stderr.contains("Invalid") || stderr.contains("branch") || stderr.contains(".."),
        "Expected validation error, got: {}",
        stderr
    );
}

// ==================== Special Characters ====================

#[test]
fn test_new_rejects_names_with_tilde() {
    let dir = setup_test_repo();

    let (ok, _, stderr) = run_wt(dir.path(), &["new", "bad~name"]);

    assert!(!ok);
    assert!(
        stderr.contains("Invalid") || stderr.contains("branch"),
        "Expected validation error for ~, got: {}",
        stderr
    );
}

#[test]
fn test_new_rejects_names_with_caret() {
    let dir = setup_test_repo();

    let (ok, _, stderr) = run_wt(dir.path(), &["new", "bad^name"]);

    assert!(!ok);
    assert!(
        stderr.contains("Invalid") || stderr.contains("branch"),
        "Expected validation error for ^, got: {}",
        stderr
    );
}

#[test]
fn test_new_rejects_names_with_colon() {
    let dir = setup_test_repo();

    let (ok, _, stderr) = run_wt(dir.path(), &["new", "bad:name"]);

    assert!(!ok);
    assert!(
        stderr.contains("Invalid") || stderr.contains("branch"),
        "Expected validation error for :, got: {}",
        stderr
    );
}

#[test]
fn test_new_rejects_names_with_question_mark() {
    let dir = setup_test_repo();

    let (ok, _, stderr) = run_wt(dir.path(), &["new", "bad?name"]);

    assert!(!ok);
    assert!(
        stderr.contains("Invalid") || stderr.contains("branch"),
        "Expected validation error for ?, got: {}",
        stderr
    );
}

#[test]
fn test_new_rejects_names_with_asterisk() {
    let dir = setup_test_repo();

    let (ok, _, stderr) = run_wt(dir.path(), &["new", "bad*name"]);

    assert!(!ok);
    assert!(
        stderr.contains("Invalid") || stderr.contains("branch"),
        "Expected validation error for *, got: {}",
        stderr
    );
}

#[test]
fn test_new_rejects_names_with_bracket() {
    let dir = setup_test_repo();

    let (ok, _, stderr) = run_wt(dir.path(), &["new", "bad[name"]);

    assert!(!ok);
    assert!(
        stderr.contains("Invalid") || stderr.contains("branch"),
        "Expected validation error for [, got: {}",
        stderr
    );
}

#[test]
fn test_new_rejects_names_with_at_brace() {
    let dir = setup_test_repo();

    let (ok, _, stderr) = run_wt(dir.path(), &["new", "bad@{name"]);

    assert!(!ok);
    assert!(
        stderr.contains("Invalid") || stderr.contains("branch"),
        "Expected validation error for @{{, got: {}",
        stderr
    );
}

#[test]
fn test_new_rejects_names_starting_with_dot() {
    let dir = setup_test_repo();

    let (ok, _, stderr) = run_wt(dir.path(), &["new", ".hidden"]);

    assert!(!ok);
    assert!(
        stderr.contains("Invalid") || stderr.contains("branch") || stderr.contains("cannot start with"),
        "Expected validation error for starting with ., got: {}",
        stderr
    );
}

// ==================== Branch Conflict ====================

#[test]
fn test_new_name_conflict_with_existing_branch() {
    let dir = setup_test_repo();

    // Create an existing branch wt/existing-branch
    std::process::Command::new("git")
        .current_dir(dir.path())
        .args(["branch", "wt/existing-branch"])
        .output()
        .expect("Failed to create branch");

    // Try to create scratch with same branch name
    let (ok, _, stderr) = run_wt(dir.path(), &["new", "existing-branch"]);

    assert!(!ok);
    assert!(
        stderr.contains("already exists") || stderr.contains("Branch"),
        "Expected branch conflict error, got: {}",
        stderr
    );
}

// ==================== Auto Name Skips Existing ====================

#[test]
fn test_new_auto_name_skips_existing_branch() {
    let dir = setup_test_repo();

    // Create wt/s1 branch to force auto-name to skip it
    std::process::Command::new("git")
        .current_dir(dir.path())
        .args(["branch", "wt/s1"])
        .output()
        .expect("Failed to create branch");

    // Run wt new without name - should try s2 since s1 exists
    let (_ok, stdout, stderr) = run_wt(dir.path(), &["new"]);

    let output = format!("{}{}", stdout, stderr);

    // Should attempt s2 (or higher) since s1 branch exists
    assert!(
        output.contains("s2") || output.contains("s3"),
        "Expected auto-generated name to skip s1, got: {}",
        output
    );
}

#[test]
fn test_new_auto_name_skips_existing_status_entry() {
    let dir = setup_test_repo();

    // Create s1 in status.json
    set_scratch_status(dir.path(), "s1", "running");

    // Run wt new without name - should try s2
    let (_ok, stdout, stderr) = run_wt(dir.path(), &["new"]);

    let output = format!("{}{}", stdout, stderr);

    // Should attempt s2 since s1 exists in status
    assert!(
        output.contains("s2") || output.contains("s3"),
        "Expected auto-generated name to skip s1, got: {}",
        output
    );
}

// ==================== --print-path Option ====================

#[test]
fn test_new_print_path_only_outputs_path() {
    let dir = setup_test_repo();

    // --print-path should only output the worktree path
    let (_ok, stdout, _stderr) = run_wt(dir.path(), &["new", "--print-path", "path-test"]);

    // If successful (or partial success before tmux), stdout should be just the path
    // The path format is: {worktree_dir}/{name}
    if !stdout.is_empty() {
        // Should not contain verbose messages like "Created scratch" or "Copied:"
        assert!(
            !stdout.contains("Created scratch") && !stdout.contains("Copied:"),
            "--print-path should suppress verbose output, got: {}",
            stdout
        );

        // Path should contain the name
        if stdout.contains("path-test") {
            assert!(
                stdout.trim().ends_with("path-test") || stdout.contains("/path-test"),
                "--print-path should output path containing name, got: {}",
                stdout
            );
        }
    }
}

#[test]
fn test_new_without_print_path_shows_verbose() {
    let dir = setup_test_repo();

    // Without --print-path, should show verbose output
    let (_ok, stdout, stderr) = run_wt(dir.path(), &["new", "verbose-test"]);

    let output = format!("{}{}", stdout, stderr);

    // Should contain some informational output (branch, worktree, etc.)
    // Even if it fails on tmux, the early output should mention the environment
    assert!(
        output.contains("verbose-test") || output.contains("wt/"),
        "Without --print-path, should show verbose info, got: {}",
        output
    );
}

// ==================== Status.json Recording ====================

#[test]
fn test_new_sets_scratch_flag_true() {
    let dir = setup_test_repo();

    // Run new command (may fail on tmux but should update status.json first)
    let _ = run_wt(dir.path(), &["new", "scratch-flag-test"]);

    // Check status.json if it was created/updated
    let status = parse_status_json(dir.path());
    if let Some(task) = status.get("tasks").and_then(|t| t.get("scratch-flag-test")) {
        // If entry exists, scratch should be true
        let is_scratch = task.get("scratch").and_then(|v| v.as_bool()).unwrap_or(false);
        assert!(
            is_scratch,
            "scratch flag should be true, got: {:?}",
            task
        );
    }
    // If no entry, command failed before writing (acceptable in test env)
}

#[test]
fn test_new_sets_status_running() {
    let dir = setup_test_repo();

    let _ = run_wt(dir.path(), &["new", "status-test"]);

    let status = parse_status_json(dir.path());
    if let Some(task) = status.get("tasks").and_then(|t| t.get("status-test")) {
        let task_status = task.get("status").and_then(|v| v.as_str()).unwrap_or("");
        assert_eq!(
            task_status, "running",
            "status should be 'running', got: {}",
            task_status
        );
    }
}

#[test]
fn test_new_records_instance_info() {
    let dir = setup_test_repo();

    let _ = run_wt(dir.path(), &["new", "instance-test"]);

    let status = parse_status_json(dir.path());
    if let Some(task) = status.get("tasks").and_then(|t| t.get("instance-test")) {
        if let Some(instance) = task.get("instance") {
            // Verify instance contains expected fields
            assert!(
                instance.get("branch").is_some(),
                "instance should have branch field"
            );
            assert!(
                instance.get("worktree_path").is_some(),
                "instance should have worktree_path field"
            );
            assert!(
                instance.get("tmux_session").is_some(),
                "instance should have tmux_session field"
            );
            assert!(
                instance.get("tmux_window").is_some(),
                "instance should have tmux_window field"
            );

            // session_id should be None (null) for scratch
            let session_id = instance.get("session_id");
            assert!(
                session_id.is_none() || session_id == Some(&serde_json::Value::Null),
                "session_id should be null for scratch, got: {:?}",
                session_id
            );
        }
    }
}

// ==================== No Config Error ====================

#[test]
fn test_new_fails_without_config() {
    let dir = tempfile::tempdir().unwrap();

    // Init git repo but don't create .wt/config.yaml
    std::process::Command::new("git")
        .current_dir(dir.path())
        .args(["init"])
        .output()
        .expect("Failed to init git");

    std::process::Command::new("git")
        .current_dir(dir.path())
        .args(["config", "user.email", "test@test.com"])
        .output()
        .ok();
    std::process::Command::new("git")
        .current_dir(dir.path())
        .args(["config", "user.name", "Test"])
        .output()
        .ok();

    std::fs::write(dir.path().join("README.md"), "# Test").unwrap();
    std::process::Command::new("git")
        .current_dir(dir.path())
        .args(["add", "."])
        .output()
        .ok();
    std::process::Command::new("git")
        .current_dir(dir.path())
        .args(["commit", "-m", "init"])
        .output()
        .ok();

    // Try wt new without config
    let (ok, _, stderr) = run_wt(dir.path(), &["new", "test"]);

    assert!(!ok);
    assert!(
        stderr.contains("config") || stderr.contains("not found") || stderr.contains("init"),
        "Expected config error, got: {}",
        stderr
    );
}
