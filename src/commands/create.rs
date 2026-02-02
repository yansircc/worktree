use crate::error::{Result, WtError};
use crate::models::{TaskInput, TaskStore};

pub fn execute(json: String) -> Result<()> {
    let input: TaskInput =
        serde_json::from_str(&json).map_err(|e| WtError::InvalidJson(e.to_string()))?;

    let file_path = TaskStore::create(&input)?;

    println!("Task '{}' created.", input.name);
    println!("  File: {}", file_path.display());

    if !input.depends.is_empty() {
        println!("  Depends: {}", input.depends.join(", "));
    }

    Ok(())
}
