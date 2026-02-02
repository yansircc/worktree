//! Tail command - view last assistant messages from task transcript.

use std::path::Path;

use serde::Serialize;

use crate::error::{Result, WtError};
use crate::models::{TaskStatus, TaskStore};
use crate::services::transcript;

#[derive(Serialize)]
struct Message {
    role: &'static str,
    content: String,
}

pub fn execute(task_ref: String, count: usize) -> Result<()> {
    let store = TaskStore::load()?;

    // Resolve task reference (name or index) to actual name
    let name = store.resolve_task_ref(&task_ref)?;

    // Check task exists
    store.ensure_exists(&name)?;

    // Check status - only Pending is not allowed
    let status = store.get_status(&name);
    if status == TaskStatus::Pending {
        return Err(WtError::TaskNotStarted(name));
    }

    // Get instance info
    let instance = store
        .get_instance(&name)
        .ok_or_else(|| WtError::TaskNotFound(name.clone()))?;

    // Check worktree exists
    let worktree_path = &instance.worktree_path;
    if !Path::new(worktree_path).exists() {
        return Err(WtError::WorktreeNotFound(name));
    }

    // Find transcript file
    let transcript_path = transcript::find_transcript_for_instance(instance)
        .ok_or_else(|| WtError::TranscriptNotFound(name.clone()))?;

    // Get last N messages
    let messages = transcript::get_last_messages(&transcript_path, count)
        .ok_or_else(|| WtError::TranscriptParseFailed(name.clone()))?;

    if messages.is_empty() {
        return Err(WtError::NoAssistantMessages(name));
    }

    // Always output JSON
    let output: Vec<Message> = messages
        .iter()
        .map(|content| Message {
            role: "assistant",
            content: content.clone(),
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}
