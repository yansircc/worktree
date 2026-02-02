use std::collections::HashMap;
use std::time::SystemTime;

use crate::constants::IDLE_THRESHOLD_SECS;
use crate::display::format_duration;
use crate::error::Result;
use crate::models::{TaskStatus, TaskStore};
use crate::services::{git, tmux, transcript};

use super::types::{StatusOutput, StatusSummary, TaskMetrics};

// ANSI color codes
const GRAY: &str = "\x1b[90m";
const RESET: &str = "\x1b[0m";

fn colored_index(idx: usize) -> String {
    format!("{}{}{}", GRAY, idx, RESET)
}

/// Display status in JSON or human-readable format
pub fn display_status(json: bool) -> Result<()> {
    let mut store = TaskStore::load()?;

    let mut metrics_list = Vec::new();
    let mut running_count = 0;
    let mut done_count = 0;
    let mut total_additions = 0;
    let mut total_deletions = 0;
    let mut status_changed = false;

    // Collect task names first to avoid borrow conflict
    let task_names: Vec<String> = store.list().iter().map(|t| t.name().to_string()).collect();

    // Build name -> index mapping (1-based)
    let index_map: HashMap<String, usize> = task_names
        .iter()
        .enumerate()
        .map(|(i, name)| (name.clone(), i + 1))
        .collect();

    for task_name in &task_names {
        // Auto-mark as Done if Running but tmux window is closed
        if store.auto_mark_done_if_needed(task_name)? {
            status_changed = true;
        }

        let status = store.get_status(task_name);

        // Only show Running and Done tasks
        if status != TaskStatus::Running && status != TaskStatus::Done {
            continue;
        }

        let instance = store.get_instance(task_name);

        // Check if tmux window is alive
        let tmux_alive = instance
            .map(|i| tmux::window_exists(&i.tmux_session, &i.tmux_window))
            .unwrap_or(false);

        let final_status = status;

        if final_status == TaskStatus::Running {
            running_count += 1;
        } else {
            done_count += 1;
        }

        let instance = store.get_instance(task_name);
        let worktree_path = instance.map(|i| i.worktree_path.as_str());

        // Get session_id and transcript path info
        let session_id = instance.and_then(|i| i.session_id.clone());

        // Find transcript file for this instance
        let transcript_path_opt = instance.and_then(transcript::find_transcript_for_instance);
        let transcript_exists = transcript_path_opt.as_ref().map(|p| p.exists());

        // Parse transcript for metrics
        let transcript_metrics = transcript_path_opt
            .as_ref()
            .and_then(|path| transcript::parse_transcript(path));

        // Duration from transcript timestamps
        let (duration_secs, duration_human) = transcript_metrics
            .as_ref()
            .and_then(|m| m.duration_secs())
            .map(|secs| (Some(secs), Some(format_duration(secs))))
            .unwrap_or((None, None));

        // Context percent and current tool from transcript
        let context_percent = transcript_metrics.as_ref().map(|m| m.context_percent());
        let current_tool = transcript_metrics.as_ref().and_then(|m| m.current_tool.clone());

        // Get git metrics (additions, deletions, commits, conflict)
        let git_metrics = worktree_path.and_then(git::get_worktree_metrics);

        if let Some(ref m) = git_metrics {
            total_additions += m.additions;
            total_deletions += m.deletions;
        }

        // tmux_alive for JSON output (only meaningful for running tasks)
        let tmux_alive_for_output = if final_status == TaskStatus::Running {
            Some(tmux_alive)
        } else {
            None
        };

        // Get idle time and activity status
        let (idle_secs, active) = if let Some(path) = worktree_path {
            if let Some(last_activity) = git::get_last_activity(path) {
                let idle = SystemTime::now()
                    .duration_since(last_activity)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let is_active = idle < IDLE_THRESHOLD_SECS;
                (Some(idle), Some(is_active))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        metrics_list.push(TaskMetrics {
            index: index_map[task_name],
            name: task_name.to_string(),
            status: final_status,
            duration_secs,
            duration_human,
            context_percent,
            current_tool,
            git: git_metrics,
            idle_secs,
            active,
            tmux_alive: tmux_alive_for_output,
            session_id,
            transcript_exists,
        });
    }

    // Save status if any task was auto-marked as Done
    if status_changed {
        store.save_status()?;
    }

    let output = StatusOutput {
        tasks: metrics_list,
        summary: StatusSummary {
            running: running_count,
            done: done_count,
            total_additions,
            total_deletions,
        },
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
    } else {
        print_human_readable(&output);
    }

    Ok(())
}

fn print_human_readable(output: &StatusOutput) {
    if output.tasks.is_empty() {
        println!("No running or done tasks.");
        return;
    }

    println!("Tasks:");
    println!();

    for task in &output.tasks {
        // tmux_alive takes precedence: if window is dead, show warning
        let status_indicator = match task.tmux_alive {
            Some(false) => " ⚠️  (tmux window closed)",
            _ => match task.active {
                Some(true) => " 🟢",
                Some(false) => " 💤",
                None => "",
            },
        };

        println!(
            "{} {} {} ({}){}",
            colored_index(task.index),
            task.status.colored_icon(),
            task.name,
            task.status.display_name(),
            status_indicator
        );

        if let Some(ref duration) = task.duration_human {
            println!("    Duration: {}", duration);
        }

        if let Some(ref git) = task.git {
            if git.additions > 0 || git.deletions > 0 {
                println!("    Changes:  +{} -{}", git.additions, git.deletions);
            }
            if git.commits > 0 {
                println!("    Commits:  {}", git.commits);
            }
        }

        println!();
    }

    println!("---");
    println!(
        "Summary: {} running, {} done | +{} -{}",
        output.summary.running,
        output.summary.done,
        output.summary.total_additions,
        output.summary.total_deletions
    );
}
