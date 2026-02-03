use std::collections::HashMap;

use crate::models::{TaskStatus, TaskStore};
use crate::tui::{App, TuiAction};

use super::types::{ActionResponse, CommandInfo, TaskInfo};

// ============================================================================
// Response Builder Helpers
// ============================================================================

/// Build a successful action response with task state transition info
fn success_response(
    action: &str,
    task_name: &str,
    before: TaskStatus,
    after: TaskStatus,
) -> ActionResponse {
    ActionResponse {
        action: action.to_string(),
        success: true,
        error: None,
        task: Some(TaskInfo {
            name: task_name.to_string(),
            status: None,
            status_before: Some(before),
            status_after: Some(after),
            mux_alive: None,
        }),
        available_actions: None,
        unavailable_actions: None,
        command: None,
    }
}

/// Build an error response for action failures
fn error_response(
    action: &str,
    error: &str,
    task_name: &str,
    status: Option<TaskStatus>,
    mux_alive: Option<bool>,
) -> ActionResponse {
    ActionResponse {
        action: action.to_string(),
        success: false,
        error: Some(error.to_string()),
        task: Some(TaskInfo {
            name: task_name.to_string(),
            status,
            status_before: None,
            status_after: None,
            mux_alive,
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
            "Task '{}' not found (only active/idle tasks are available)",
            task_name
        )),
        task: Some(TaskInfo {
            name: task_name.to_string(),
            status: None,
            status_before: None,
            status_after: None,
            mux_alive: None,
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
        None => respond_and_exit(error_response_no_task(
            action,
            "--task is required with --action",
        )),
    };

    // Resolve task reference (name or index) to actual name
    let store = match TaskStore::load() {
        Ok(s) => s,
        Err(e) => respond_and_exit(error_response_no_task(
            action,
            &format!("Failed to load tasks: {}", e),
        )),
    };

    let task_name = match store.resolve_task_ref(&task_ref) {
        Ok(name) => name,
        Err(e) => respond_and_exit(error_response_no_task(action, &e.to_string())),
    };

    let mut app = match App::new(false) {
        Ok(app) => app,
        Err(e) => respond_and_exit(error_response_no_task(
            action,
            &format!("Failed to initialize: {}", e),
        )),
    };

    let task_idx = match app.tasks.iter().position(|t| t.name == task_name) {
        Some(idx) => idx,
        None => respond_and_exit(task_not_found_response(action, &task_name)),
    };

    app.selected = task_idx;

    let response = match action {
        "list" => handle_list_action(&app, &task_name),
        "stop" | "review" | "done" => handle_stop_action(&task_name),
        "next" | "resume" => handle_next_action(&task_name),
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
                mux_alive: None,
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

    // tail/enter available for Active/Idle
    if matches!(task.status, TaskStatus::Active | TaskStatus::Idle) {
        available.push("tail".to_string());
        available.push("enter".to_string());
    } else {
        unavailable.insert(
            "tail".to_string(),
            format!(
                "task is {} (need active or idle)",
                task.status.display_name()
            ),
        );
        unavailable.insert(
            "enter".to_string(),
            format!(
                "task is {} (need active or idle)",
                task.status.display_name()
            ),
        );
    }

    // stop available for Active
    if task.status == TaskStatus::Active {
        available.push("stop".to_string());
    } else {
        unavailable.insert(
            "stop".to_string(),
            format!("task is {} (need active)", task.status.display_name()),
        );
    }

    // next available for non-Completed
    if task.status != TaskStatus::Completed {
        available.push("next".to_string());
    } else {
        unavailable.insert(
            "next".to_string(),
            "task is already completed".to_string(),
        );
    }

    ActionResponse {
        action: "list".to_string(),
        success: true,
        error: None,
        task: Some(TaskInfo {
            name: task_name.to_string(),
            status: Some(task.status),
            status_before: None,
            status_after: None,
            mux_alive: Some(task.mux_alive),
        }),
        available_actions: Some(available),
        unavailable_actions: Some(unavailable),
        command: None,
    }
}

fn handle_stop_action(task_name: &str) -> ActionResponse {
    let store = match TaskStore::load() {
        Ok(s) => s,
        Err(e) => {
            return error_response(
                "stop",
                &format!("Failed to load tasks: {}", e),
                task_name,
                None,
                None,
            )
        }
    };

    let status_before = store.get_status(task_name);

    if status_before != TaskStatus::Active {
        return error_response(
            "stop",
            &format!(
                "Cannot stop: task is {} (need active)",
                status_before.display_name()
            ),
            task_name,
            Some(status_before),
            None,
        );
    }

    // Execute wt stop
    if let Err(e) = crate::commands::stop::execute(task_name.to_string(), false) {
        return error_response(
            "stop",
            &format!("Failed to stop: {}", e),
            task_name,
            Some(status_before),
            None,
        );
    }

    success_response("stop", task_name, status_before, TaskStatus::Idle)
}

fn handle_next_action(task_name: &str) -> ActionResponse {
    let store = match TaskStore::load() {
        Ok(s) => s,
        Err(e) => {
            return error_response(
                "next",
                &format!("Failed to load tasks: {}", e),
                task_name,
                None,
                None,
            )
        }
    };

    let status_before = store.get_status(task_name);

    if status_before == TaskStatus::Completed {
        return error_response(
            "next",
            "Cannot advance: task is already completed",
            task_name,
            Some(status_before),
            None,
        );
    }

    // Execute wt next
    if let Err(e) = crate::commands::next::execute(task_name.to_string()) {
        return error_response(
            "next",
            &format!("Failed to advance: {}", e),
            task_name,
            Some(status_before),
            None,
        );
    }

    // Reload to get new status
    let new_store = TaskStore::load().ok();
    let status_after = new_store
        .map(|s| s.get_status(task_name))
        .unwrap_or(TaskStatus::Active);

    success_response("next", task_name, status_before, status_after)
}

fn handle_enter_action(app: &App, task_name: &str) -> ActionResponse {
    let task = app.selected_task().unwrap();

    match app.enter_action() {
        Some(TuiAction::SwitchWindow {
            session, window, ..
        })
        | Some(TuiAction::AttachSession {
            session, window, ..
        }) => ActionResponse {
            action: "enter".to_string(),
            success: true,
            error: None,
            task: Some(TaskInfo {
                name: task_name.to_string(),
                status: None,
                status_before: None,
                status_after: None,
                mux_alive: None,
            }),
            available_actions: None,
            unavailable_actions: None,
            command: Some(CommandInfo {
                cmd_type: "mux_switch".to_string(),
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
                mux_alive: None,
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
            error: Some("Cannot enter: no multiplexer info available".to_string()),
            task: Some(TaskInfo {
                name: task_name.to_string(),
                status: Some(task.status),
                status_before: None,
                status_after: None,
                mux_alive: Some(task.mux_alive),
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
        Err(e) => error_response(
            "tail",
            &format!("Failed to tail: {}", e),
            task_name,
            None,
            None,
        ),
    }
}
