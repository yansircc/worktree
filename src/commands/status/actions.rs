use std::collections::HashMap;

use crate::models::{TaskStatus, TaskStore, UserAction};
use crate::services::action_resolver::{resolve_enter_action, TaskActionContext};

use super::types::{ActionResponse, CommandInfo, TaskInfo};

// ============================================================================
// Action Execution
// ============================================================================

/// Execute an action via the --action API
pub fn execute_action(action: &str, task_ref: Option<String>) {
    let task_ref = match task_ref {
        Some(r) => r,
        None => ActionResponse::error_no_task(action, "--task is required with --action")
            .print_and_exit(),
    };

    // Resolve task reference (name or index) to actual name
    let store = match TaskStore::load() {
        Ok(s) => s,
        Err(e) => ActionResponse::error_no_task(action, format!("Failed to load tasks: {}", e))
            .print_and_exit(),
    };

    let task_name = match store.resolve_task_ref(&task_ref) {
        Ok(name) => name,
        Err(e) => ActionResponse::error_no_task(action, e.to_string()).print_and_exit(),
    };

    // Build action context directly from store (no TUI dependency)
    let status = store.get_status(&task_name);
    let instance = store.get_instance(&task_name);
    let ctx = TaskActionContext::from_store(&task_name, status, instance);

    // Only allow actions on active/idle tasks (same as TUI filter)
    if !matches!(status, TaskStatus::Active | TaskStatus::Idle | TaskStatus::Pending) {
        ActionResponse::task_not_found(action, &task_name).print_and_exit();
    }

    let response = match action {
        "list" => handle_list_action(&ctx),
        "stop" | "review" | "done" => handle_stop_action(&task_name),
        "next" | "resume" => handle_next_action(&task_name),
        "enter" => handle_enter_action(&ctx),
        "tail" => handle_tail_action(&task_name),
        _ => ActionResponse::unknown_action(action, &task_name),
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&response).unwrap_or_default()
    );

    if !response.success {
        std::process::exit(1);
    }
}

fn handle_list_action(ctx: &TaskActionContext) -> ActionResponse {
    let mut available = vec![];
    let mut unavailable = HashMap::new();

    // tail/enter available for Active/Idle
    if matches!(ctx.status, TaskStatus::Active | TaskStatus::Idle) {
        available.push("tail".to_string());
        available.push("enter".to_string());
    } else {
        unavailable.insert(
            "tail".to_string(),
            format!(
                "task is {} (need active or idle)",
                ctx.status.display_name()
            ),
        );
        unavailable.insert(
            "enter".to_string(),
            format!(
                "task is {} (need active or idle)",
                ctx.status.display_name()
            ),
        );
    }

    // stop available for Active
    if ctx.status == TaskStatus::Active {
        available.push("stop".to_string());
    } else {
        unavailable.insert(
            "stop".to_string(),
            format!("task is {} (need active)", ctx.status.display_name()),
        );
    }

    // next available for non-Completed
    if ctx.status != TaskStatus::Completed {
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
        task: Some(TaskInfo::with_status(&ctx.name, ctx.status, ctx.mux_alive)),
        available_actions: Some(available),
        unavailable_actions: Some(unavailable),
        command: None,
    }
}

fn handle_stop_action(task_name: &str) -> ActionResponse {
    let store = match TaskStore::load() {
        Ok(s) => s,
        Err(e) => {
            return ActionResponse::error(
                "stop",
                format!("Failed to load tasks: {}", e),
                task_name,
                None,
                None,
            )
        }
    };

    let status_before = store.get_status(task_name);

    if status_before != TaskStatus::Active {
        return ActionResponse::error(
            "stop",
            format!(
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
        return ActionResponse::error(
            "stop",
            format!("Failed to stop: {}", e),
            task_name,
            Some(status_before),
            None,
        );
    }

    ActionResponse::success("stop", task_name, status_before, TaskStatus::Idle)
}

fn handle_next_action(task_name: &str) -> ActionResponse {
    let store = match TaskStore::load() {
        Ok(s) => s,
        Err(e) => {
            return ActionResponse::error(
                "next",
                format!("Failed to load tasks: {}", e),
                task_name,
                None,
                None,
            )
        }
    };

    let status_before = store.get_status(task_name);

    if status_before == TaskStatus::Completed {
        return ActionResponse::error(
            "next",
            "Cannot advance: task is already completed",
            task_name,
            Some(status_before),
            None,
        );
    }

    // Execute wt next
    if let Err(e) = crate::commands::next::execute(task_name.to_string()) {
        return ActionResponse::error(
            "next",
            format!("Failed to advance: {}", e),
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

    ActionResponse::success("next", task_name, status_before, status_after)
}

fn handle_enter_action(ctx: &TaskActionContext) -> ActionResponse {
    let task_info = TaskInfo::name_only(&ctx.name);

    match resolve_enter_action(ctx) {
        Some(UserAction::SwitchWindow {
            session, window, ..
        })
        | Some(UserAction::AttachSession {
            session, window, ..
        }) => ActionResponse {
            action: "enter".to_string(),
            success: true,
            error: None,
            task: Some(task_info),
            available_actions: None,
            unavailable_actions: None,
            command: Some(CommandInfo {
                cmd_type: "mux_switch".to_string(),
                session: Some(session),
                window: Some(window),
                ..Default::default()
            }),
        },
        Some(UserAction::ShowResume {
            worktree,
            session_id,
            claude_command,
        }) => ActionResponse {
            action: "enter".to_string(),
            success: true,
            error: None,
            task: Some(task_info),
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
        Some(UserAction::OpenWorktreeShell {
            session,
            worktree_path,
            ..
        }) => ActionResponse {
            action: "enter".to_string(),
            success: true,
            error: None,
            task: Some(task_info),
            available_actions: None,
            unavailable_actions: None,
            command: Some(CommandInfo {
                cmd_type: "open_shell".to_string(),
                session: Some(session),
                worktree: Some(worktree_path),
                ..Default::default()
            }),
        },
        _ => ActionResponse {
            action: "enter".to_string(),
            success: false,
            error: Some("Cannot enter: no multiplexer info available".to_string()),
            task: Some(TaskInfo::with_status(&ctx.name, ctx.status, ctx.mux_alive)),
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
        Err(e) => ActionResponse::error(
            "tail",
            format!("Failed to tail: {}", e),
            task_name,
            None,
            None,
        ),
    }
}
