use std::collections::HashMap;

use serde::Serialize;

use crate::display::colored_index;
use crate::error::Result;
use crate::models::{Task, TaskStatus, TaskStore};

#[derive(Serialize)]
struct NextOutput {
    ready: Vec<TaskWithIndex>,
    blocked: Vec<BlockedTask>,
}

#[derive(Serialize)]
struct TaskWithIndex {
    index: usize,
    name: String,
}

#[derive(Serialize)]
struct BlockedTask {
    index: usize,
    name: String,
    waiting_for: Vec<String>,
}

pub fn execute(json: bool) -> Result<()> {
    let store = TaskStore::load()?;
    let tasks = store.list();

    // Build name -> index mapping (1-based)
    let index_map: HashMap<&str, usize> = tasks
        .iter()
        .enumerate()
        .map(|(i, t)| (t.name(), i + 1))
        .collect();

    let (ready, blocked) = classify_tasks(&tasks, &store);

    if json {
        print_json(&ready, &blocked, &index_map);
    } else {
        print_human(&ready, &blocked, &index_map);
    }

    Ok(())
}

fn classify_tasks<'a>(
    tasks: &[&'a Task],
    store: &TaskStore,
) -> (Vec<&'a Task>, Vec<(&'a Task, Vec<String>)>) {
    let mut ready = Vec::new();
    let mut blocked = Vec::new();

    for task in tasks {
        // Skip tasks that are not pending (get status from StatusStore)
        if store.get_status(task.name()) != TaskStatus::Pending {
            continue;
        }

        let unmerged_deps: Vec<String> = task
            .depends()
            .iter()
            .filter(|dep_name| store.get_status(dep_name) != TaskStatus::Merged)
            .cloned()
            .collect();

        if unmerged_deps.is_empty() {
            ready.push(*task);
        } else {
            blocked.push((*task, unmerged_deps));
        }
    }

    (ready, blocked)
}

fn print_json(ready: &[&Task], blocked: &[(&Task, Vec<String>)], index_map: &HashMap<&str, usize>) {
    let output = NextOutput {
        ready: ready
            .iter()
            .map(|t| TaskWithIndex {
                index: index_map[t.name()],
                name: t.name().to_string(),
            })
            .collect(),
        blocked: blocked
            .iter()
            .map(|(t, deps)| BlockedTask {
                index: index_map[t.name()],
                name: t.name().to_string(),
                waiting_for: deps.clone(),
            })
            .collect(),
    };
    println!("{}", serde_json::to_string(&output).unwrap_or_default());
}

fn print_human(ready: &[&Task], blocked: &[(&Task, Vec<String>)], index_map: &HashMap<&str, usize>) {
    if ready.is_empty() && blocked.is_empty() {
        println!("No pending tasks.");
        return;
    }

    if !ready.is_empty() {
        println!("Ready to start:");
        for task in ready {
            let idx = index_map[task.name()];
            println!("  {} {} {}", colored_index(idx), TaskStatus::Pending.colored_icon(), task.name());
        }
    }

    if !blocked.is_empty() {
        if !ready.is_empty() {
            println!();
        }
        println!("Blocked:");
        for (task, deps) in blocked {
            let idx = index_map[task.name()];
            println!("  {} {} {} (waiting for: {})", colored_index(idx), TaskStatus::Pending.colored_icon(), task.name(), deps.join(", "));
        }
    }
}
