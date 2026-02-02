use crate::constants::TASKS_DIR;
use crate::error::Result;
use crate::models::TaskStore;

pub fn execute(task_ref: Option<String>) -> Result<()> {
    let store = TaskStore::load()?;

    // Resolve task reference to name if provided
    let name = match task_ref {
        Some(ref r) => Some(store.resolve_task_ref(r)?),
        None => None,
    };

    // Task existence already checked by resolve_task_ref

    if store.tasks.is_empty() {
        println!("No tasks found in {}/", TASKS_DIR);
        return Ok(());
    }

    let errors = store.validate();

    // Filter by name if specified
    let errors: Vec<_> = if let Some(ref name) = name {
        errors
            .into_iter()
            .filter(|(n, _)| n == name || n.contains(name))
            .collect()
    } else {
        errors
    };

    if errors.is_empty() {
        let count = if name.is_some() { 1 } else { store.tasks.len() };
        println!("✓ All {} task(s) valid.", count);
    } else {
        for (task, error) in &errors {
            println!("✗ {}: {}", task, error);
        }
        println!();
        println!("{} error(s) found.", errors.len());
    }

    Ok(())
}
