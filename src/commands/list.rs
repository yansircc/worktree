use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::constants::TASKS_DIR;
use crate::display::colored_index;
use crate::error::Result;
use crate::models::{Task, TaskStatus, TaskStore};

#[derive(Serialize)]
struct TaskJson {
    index: usize,
    name: String,
    status: TaskStatus,
    depends: Vec<String>,
}

#[derive(Serialize)]
struct ListOutput {
    tasks: Vec<TaskJson>,
}

pub fn execute(tree: bool, json: bool) -> Result<()> {
    let store = TaskStore::load()?;
    let tasks = store.list();

    if json {
        print_json(&tasks, &store);
    } else if tree {
        print_tree(&tasks, &store);
    } else {
        print_grouped(&tasks, &store);
    }

    Ok(())
}

fn print_json(tasks: &[&Task], store: &TaskStore) {
    let output = ListOutput {
        tasks: tasks
            .iter()
            .enumerate()
            .map(|(i, t)| TaskJson {
                index: i + 1,
                name: t.name().to_string(),
                status: store.get_status(t.name()),
                depends: t.depends().to_vec(),
            })
            .collect(),
    };
    println!("{}", serde_json::to_string(&output).unwrap_or_default());
}

fn print_grouped(tasks: &[&Task], store: &TaskStore) {
    if tasks.is_empty() {
        println!("No tasks found in {}/", TASKS_DIR);
        return;
    }

    // Build name -> index mapping (1-based)
    let index_map: HashMap<&str, usize> = tasks
        .iter()
        .enumerate()
        .map(|(i, t)| (t.name(), i + 1))
        .collect();

    // Group tasks by status (with index)
    let mut archived: Vec<(usize, &Task)> = Vec::new();
    let mut merged: Vec<(usize, &Task)> = Vec::new();
    let mut ready: Vec<(usize, &Task)> = Vec::new();
    let mut blocked: Vec<(usize, &Task, Vec<&str>)> = Vec::new();
    let mut running: Vec<(usize, &Task)> = Vec::new();
    let mut done: Vec<(usize, &Task)> = Vec::new();

    for task in tasks {
        let idx = index_map[task.name()];
        let status = store.get_status(task.name());
        match status {
            TaskStatus::Archived => archived.push((idx, task)),
            TaskStatus::Merged => merged.push((idx, task)),
            TaskStatus::Running => running.push((idx, task)),
            TaskStatus::Done => done.push((idx, task)),
            TaskStatus::Pending => {
                // Check if all dependencies are merged or archived
                let unmerged_deps: Vec<&str> = task
                    .depends()
                    .iter()
                    .filter(|dep| {
                        let dep_status = store.get_status(dep);
                        dep_status != TaskStatus::Merged && dep_status != TaskStatus::Archived
                    })
                    .map(|s| s.as_str())
                    .collect();

                if unmerged_deps.is_empty() {
                    ready.push((idx, task));
                } else {
                    blocked.push((idx, task, unmerged_deps));
                }
            }
        }
    }

    // Print Archived
    if !archived.is_empty() {
        println!("Archived ({}):", archived.len());
        for (idx, task) in &archived {
            println!("  {} {} {}", colored_index(*idx), TaskStatus::Archived.colored_icon(), task.name());
        }
        println!();
    }

    // Print Merged
    if !merged.is_empty() {
        println!("Merged ({}):", merged.len());
        for (idx, task) in &merged {
            println!("  {} {} {}", colored_index(*idx), TaskStatus::Merged.colored_icon(), task.name());
        }
        println!();
    }

    // Print Running
    if !running.is_empty() {
        println!("Running ({}):", running.len());
        for (idx, task) in &running {
            print_task_with_deps_indexed(*idx, task, store, &index_map);
        }
        println!();
    }

    // Print Done
    if !done.is_empty() {
        println!("Done ({}):", done.len());
        for (idx, task) in &done {
            print_task_with_deps_indexed(*idx, task, store, &index_map);
        }
        println!();
    }

    // Print Ready
    if !ready.is_empty() {
        println!("Ready ({}):", ready.len());
        for (idx, task) in &ready {
            print_task_with_deps_indexed(*idx, task, store, &index_map);
        }
        println!();
    }

    // Print Blocked
    if !blocked.is_empty() {
        println!("Blocked ({}):", blocked.len());
        for (idx, task, waiting_for) in &blocked {
            print!("  {} {} {}", colored_index(*idx), TaskStatus::Pending.colored_icon(), task.name());
            if !task.depends().is_empty() {
                print!(" ←");
                for (i, dep) in task.depends().iter().enumerate() {
                    if i > 0 {
                        print!(",");
                    }
                    let icon = if waiting_for.contains(&dep.as_str()) {
                        TaskStatus::Pending.colored_icon()
                    } else {
                        TaskStatus::Merged.colored_icon()
                    };
                    print!(" {}{}", dep, icon);
                }
            }
            println!();
        }
    }
}

fn print_task_with_deps_indexed(idx: usize, task: &Task, store: &TaskStore, index_map: &HashMap<&str, usize>) {
    let status = store.get_status(task.name());
    print!("  {} {} {}", colored_index(idx), status.colored_icon(), task.name());
    if !task.depends().is_empty() {
        print!(" ←");
        for (i, dep) in task.depends().iter().enumerate() {
            if i > 0 {
                print!(",");
            }
            let dep_icon = store.get_status(dep).colored_icon();
            // Show dependency index if available
            let dep_idx_str = index_map
                .get(dep.as_str())
                .map(|idx| format!("[{}]", idx))
                .unwrap_or_default();
            print!(" {}{}{}", dep, dep_idx_str, dep_icon);
        }
    }
    println!();
}

fn print_tree(tasks: &[&Task], store: &TaskStore) {
    if tasks.is_empty() {
        println!("No tasks found in {}/", TASKS_DIR);
        return;
    }

    let children: HashMap<&str, Vec<&Task>> = build_children_map(tasks);
    let roots: Vec<&Task> = tasks
        .iter()
        .filter(|t| t.depends().is_empty())
        .copied()
        .collect();

    let mut visited = HashSet::new();
    for root in &roots {
        print_tree_node(root, &children, store, "", true, true, None, &mut visited);
    }

    for task in tasks {
        if !visited.contains(task.name()) {
            print_tree_node(task, &children, store, "", true, true, None, &mut visited);
        }
    }
}

fn build_children_map<'a>(tasks: &'a [&Task]) -> HashMap<&'a str, Vec<&'a Task>> {
    let mut map: HashMap<&str, Vec<&Task>> = HashMap::new();
    for task in tasks {
        for dep in task.depends() {
            map.entry(dep.as_str()).or_default().push(task);
        }
    }
    map
}

fn print_tree_node<'a>(
    task: &'a Task,
    children: &HashMap<&str, Vec<&'a Task>>,
    store: &TaskStore,
    prefix: &str,
    is_last: bool,
    is_root: bool,
    parent_name: Option<&str>,
    visited: &mut HashSet<&'a str>,
) {
    if visited.contains(task.name()) {
        return;
    }
    visited.insert(task.name());

    let connector = if is_root {
        ""
    } else if is_last {
        "└── "
    } else {
        "├── "
    };

    // Calculate other dependencies not shown in current tree path
    let other_deps: Vec<&str> = if let Some(parent) = parent_name {
        task.depends()
            .iter()
            .filter(|dep| dep.as_str() != parent)
            .map(|s| s.as_str())
            .collect()
    } else {
        vec![]
    };

    let other_deps_str = if other_deps.is_empty() {
        String::new()
    } else {
        format!(" (+{})", other_deps.join(", "))
    };

    let status = store.get_status(task.name());
    println!(
        "{}{}{} [{}]{}",
        prefix,
        connector,
        task.name(),
        status.colored_icon(),
        other_deps_str
    );

    let new_prefix = if is_root {
        "".to_string()
    } else if is_last {
        format!("{}    ", prefix)
    } else {
        format!("{}│   ", prefix)
    };

    if let Some(task_children) = children.get(task.name()) {
        let count = task_children.len();
        for (i, child) in task_children.iter().enumerate() {
            let is_last_child = i == count - 1;
            print_tree_node(child, children, store, &new_prefix, is_last_child, false, Some(task.name()), visited);
        }
    }
}
