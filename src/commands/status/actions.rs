use std::collections::HashMap;

use crate::models::{TaskStatus, TaskStore};
use crate::tui::{App, TuiAction};

use super::types::{ActionResponse, CommandInfo, TaskInfo};

// ============================================================================
// Response Builder Helpers
// ============================================================================

/// Build a successful action response with task state transition info
fn success_response(action: &str, task_name: &str, before: TaskStatus, after: TaskStatus) -> ActionResponse {
    ActionResponse {
        action: action.to_string(),
        success: true,
        error: None,
        task: Some(TaskInfo {
            name: task_name.to_string(),
            status: None,
            status_before: Some(before),
            status_after: Some(after),
            tmux_alive: None,
        }),
        available_actions: None,
        unavailable_actions: None,
        command: None,
    }
}

/// Build an error response for action failures
fn error_response(action: &str, error: &str, task_name: &str, status: Option<TaskStatus>, tmux_alive: Option<bool>) -> ActionResponse {
    ActionResponse {
        action: action.to_string(),
        success: false,
        error: Some(error.to_string()),
        task: Some(TaskInfo {
            name: task_name.to_string(),
            status,
            status_before: None,
            status_after: None,
            tmux_alive,
        }),
        available_actions: None,
        unavailable_actions: None,
        command: None,
    }
}

/// Build an error response without task info (for early failures)
fn error_response_no_task(action: &str, error: &str) -> ActionResponse {
    ActionResponse {
        action: action.to_string(),
        success: false,
        error: Some(error.to_string()),
        task: None,
        available_actions: None,
        unavailable_actions: None,
        command: None,
    }
}

/// Build a "task not found" error response
fn task_not_found_response(action: &str, task_name: &str) -> ActionResponse {
    ActionResponse {
        action: action.to_string(),
        success: false,
        error: Some(format!(
            "Task '{}' not found (only running/done/merged tasks are available)",
            task_name
        )),
        task: Some(TaskInfo {
            name: task_name.to_string(),
            status: None,
            status_before: None,
            status_after: None,
            tmux_alive: None,
        }),
        available_actions: None,
        unavailable_actions: None,
        command: None,
    }
}

/// Print response as JSON and exit with appropriate code
fn respond_and_exit(response: ActionResponse) -> ! {
    println!(
        "{}",
        serde_json::to_string_pretty(&response).unwrap_or_default()
    );
    std::process::exit(if response.success { 0 } else { 1 });
}

// ============================================================================
// Action Execution
// ============================================================================

/// Execute an action via the --action API
pub fn execute_action(action: &str, task_ref: Option<String>) {
    let task_ref = match task_ref {
        Some(r) => r,
        None => respond_and_exit(error_response_no_task(action, "--task is required with --action")),
    };

    // Resolve task reference (name or index) to actual name
    let store = match TaskStore::load() {
        Ok(s) => s,
        Err(e) => respond_and_exit(error_response_no_task(action, &format!("Failed to load tasks: {}", e))),
    };

    let task_name = match store.resolve_task_ref(&task_ref) {
        Ok(name) => name,
        Err(e) => respond_and_exit(error_response_no_task(action, &e.to_string())),
    };

    let mut app = match App::new() {
        Ok(app) => app,
        Err(e) => respond_and_exit(error_response_no_task(action, &format!("Failed to initialize: {}", e))),
    };

    let task_idx = match app.tasks.iter().position(|t| t.name == task_name) {
        Some(idx) => idx,
        None => respond_and_exit(task_not_found_response(action, &task_name)),
    };

    app.selected = task_idx;

    let response = match action {
        "list" => handle_list_action(&app, &task_name),
        "done" => handle_done_action(&mut app, &task_name),
        "merged" => handle_merged_action(&mut app, &task_name),
        "archive" => handle_archive_action(&mut app, &task_name),
        "enter" => handle_enter_action(&app, &task_name),
        "tail" => handle_tail_action(&task_name),
        _ => ActionResponse {
            action: action.to_string(),
            success: false,
            error: Some(format!("Unknown action: {}", action)),
            task: Some(TaskInfo {
                name: task_name,
                status: None,
                status_before: None,
                status_after: None,
                tmux_alive: None,
            }),
            available_actions: None,
            unavailable_actions: None,
            command: None,
        },
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&response).unwrap_or_default()
    );

    if !response.success {
        std::process::exit(1);
    }
}

fn handle_list_action(app: &App, task_name: &str) -> ActionResponse {
    let task = app.selected_task().unwrap();

    let mut available = vec![];
    let mut unavailable = HashMap::new();

    // tail/enter available for Running/Done
    if matches!(task.status, TaskStatus::Running | TaskStatus::Done) {
        available.push("tail".to_string());
        available.push("enter".to_string());
    } else {
        unavailable.insert(
            "tail".to_string(),
            format!("task is {} (need running or done)", task.status.display_name()),
        );
        unavailable.insert(
            "enter".to_string(),
            format!("task is {} (need running or done)", task.status.display_name()),
        );
    }

    // done check
    if app.can_mark_done() {
        available.push("done".to_string());
    } else {
        unavailable.insert(
            "done".to_string(),
            format!("task is {} (need running)", task.status.display_name()),
        );
    }

    // merged check
    if app.can_mark_merged() {
        available.push("merged".to_string());
    } else {
        unavailable.insert(
            "merged".to_string(),
            format!("task is {} (need done)", task.status.display_name()),
        );
    }

    // archive check
    if app.can_archive() {
        available.push("archive".to_string());
    } else {
        unavailable.insert(
            "archive".to_string(),
            format!("task is {} (need merged)", task.status.display_name()),
        );
    }

    ActionResponse {
        action: "list".to_string(),
        success: true,
        error: None,
        task: Some(TaskInfo {
            name: task_name.to_string(),
            status: Some(task.status.clone()),
            status_before: None,
            status_after: None,
            tmux_alive: Some(task.tmux_alive),
        }),
        available_actions: Some(available),
        unavailable_actions: Some(unavailable),
        command: None,
    }
}

fn handle_done_action(app: &mut App, task_name: &str) -> ActionResponse {
    let task = app.selected_task().unwrap();
    let status_before = task.status.clone();
    let tmux_alive = task.tmux_alive;

    if !app.can_mark_done() {
        return error_response("done", "Cannot mark as done: task is not running", task_name, Some(status_before), Some(tmux_alive));
    }

    if let Err(e) = app.mark_done() {
        return error_response("done", &format!("Failed to mark as done: {}", e), task_name, Some(status_before), None);
    }

    success_response("done", task_name, status_before, TaskStatus::Done)
}

fn handle_merged_action(app: &mut App, task_name: &str) -> ActionResponse {
    let task = app.selected_task().unwrap();
    let status_before = task.status.clone();

    if !app.can_mark_merged() {
        return error_response(
            "merged",
            &format!("Cannot mark as merged: task is {} (need done)", status_before.display_name()),
            task_name,
            Some(status_before),
            None,
        );
    }

    if let Err(e) = app.mark_merged() {
        return error_response("merged", &format!("Failed to mark as merged: {}", e), task_name, Some(status_before), None);
    }

    success_response("merged", task_name, status_before, TaskStatus::Merged)
}

fn handle_archive_action(app: &mut App, task_name: &str) -> ActionResponse {
    let task = app.selected_task().unwrap();
    let status_before = task.status.clone();

    if !app.can_archive() {
        return error_response(
            "archive",
            &format!("Cannot archive: task is {} (need merged)", status_before.display_name()),
            task_name,
            Some(status_before),
            None,
        );
    }

    if let Err(e) = app.archive() {
        return error_response("archive", &format!("Failed to archive: {}", e), task_name, Some(status_before), None);
    }

    success_response("archive", task_name, status_before, TaskStatus::Archived)
}

fn handle_enter_action(app: &App, task_name: &str) -> ActionResponse {
    let task = app.selected_task().unwrap();

    match app.enter_action() {
        Some(TuiAction::SwitchTmuxWindow { session, window })
        | Some(TuiAction::AttachTmux { session, window }) => ActionResponse {
            action: "enter".to_string(),
            success: true,
            error: None,
            task: Some(TaskInfo {
                name: task_name.to_string(),
                status: None,
                status_before: None,
                status_after: None,
                tmux_alive: None,
            }),
            available_actions: None,
            unavailable_actions: None,
            command: Some(CommandInfo {
                cmd_type: "tmux_switch".to_string(),
                session: Some(session),
                window: Some(window),
                ..Default::default()
            }),
        },
        Some(TuiAction::ShowResume {
            worktree,
            session_id,
            claude_command,
        }) => ActionResponse {
            action: "enter".to_string(),
            success: true,
            error: None,
            task: Some(TaskInfo {
                name: task_name.to_string(),
                status: None,
                status_before: None,
                status_after: None,
                tmux_alive: None,
            }),
            available_actions: None,
            unavailable_actions: None,
            command: Some(CommandInfo {
                cmd_type: "resume".to_string(),
                worktree: Some(worktree.clone()),
                session_id: Some(session_id.clone()),
                shell_command: Some(format!(
                    "cd {} && {} -r {}",
                    worktree, claude_command, session_id
                )),
                ..Default::default()
            }),
        },
        _ => ActionResponse {
            action: "enter".to_string(),
            success: false,
            error: Some("Cannot enter: no tmux info available".to_string()),
            task: Some(TaskInfo {
                name: task_name.to_string(),
                status: Some(task.status.clone()),
                status_before: None,
                status_after: None,
                tmux_alive: Some(task.tmux_alive),
            }),
            available_actions: None,
            unavailable_actions: None,
            command: None,
        },
    }
}

fn handle_tail_action(task_name: &str) -> ActionResponse {
    // Execute tail command directly - it outputs JSON
    match crate::commands::tail::execute(task_name.to_string(), 1) {
        Ok(_) => {
            // tail::execute already printed output, exit without additional JSON
            std::process::exit(0);
        }
        Err(e) => error_response("tail", &format!("Failed to tail: {}", e), task_name, None, None),
    }
}
