//! Tail command - view last assistant messages from task transcript.

use serde::Serialize;

use crate::error::{Result, WtError};
use crate::models::TaskStatus;
use crate::services::{transcript, TaskContext};

#[derive(Serialize)]
struct Message {
    role: &'static str,
    content: String,
}

pub fn execute(task_ref: String, count: usize) -> Result<()> {
    let ctx = TaskContext::load_with_task_file(&task_ref)?;

    // Check status - only Pending is not allowed
    if ctx.status() == TaskStatus::Pending {
        return Err(WtError::TaskNotStarted(ctx.name().to_string()));
    }

    // Get instance info (use TaskNotFound for backward compat)
    let instance = ctx
        .instance()
        .ok_or_else(|| WtError::TaskNotFound(ctx.name().to_string()))?;

    // Check worktree exists
    if let Some(ref wt_path) = instance.worktree_path {
        if !std::path::Path::new(wt_path).exists() {
            return Err(WtError::WorktreeNotFound(ctx.name().to_string()));
        }
    }

    // Find transcript file
    let transcript_path = transcript::find_transcript_for_instance(instance)
        .ok_or_else(|| WtError::TranscriptNotFound(ctx.name().to_string()))?;

    // Get last N messages
    let messages = transcript::get_last_messages(&transcript_path, count)
        .ok_or_else(|| WtError::TranscriptParseFailed(ctx.name().to_string()))?;

    if messages.is_empty() {
        return Err(WtError::NoAssistantMessages(ctx.name().to_string()));
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
