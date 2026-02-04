//! Logs command - extract filtered transcripts for all tasks.

use crate::error::Result;
use crate::models::{TaskStatus, TaskStore, WtConfig};
use crate::services::transcript;

pub fn execute() -> Result<()> {
    let config = WtConfig::load()?;
    let store = TaskStore::load()?;

    let mut generated = 0;
    let mut skipped = 0;

    for task in store.list() {
        let status = store.status.get_status(task.name());

        // Skip pending tasks (no transcript)
        if status == TaskStatus::Pending {
            continue;
        }

        // Get instance info
        let instance = match store.status.get_instance(task.name()) {
            Some(i) => i,
            None => {
                skipped += 1;
                continue;
            }
        };

        // Find transcript file - try session_id first, fall back to latest
        let worktree_path = match instance.worktree_path.as_deref() {
            Some(p) => p,
            None => {
                skipped += 1;
                continue;
            }
        };
        let transcript_path = instance
            .session_id
            .as_ref()
            .and_then(|sid| transcript::transcript_path(worktree_path, sid))
            .filter(|p| p.exists())
            .or_else(|| transcript::find_latest_transcript(worktree_path));

        let transcript_path = match transcript_path {
            Some(p) => p,
            None => {
                skipped += 1;
                continue;
            }
        };

        // Extract session_id from transcript path for log naming
        let session_id = transcript_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // Generate log file
        let log_path = transcript::log_path(task.name(), session_id);

        match transcript::extract_to_log(
            &transcript_path,
            &log_path,
            &config.logs.exclude_types,
            &config.logs.exclude_fields,
        ) {
            Some(count) => {
                println!(
                    "  {} -> {} ({} entries)",
                    task.name(),
                    log_path.display(),
                    count
                );
                generated += 1;
            }
            None => {
                skipped += 1;
            }
        }
    }

    println!();
    println!("Generated: {}, Skipped: {}", generated, skipped);

    Ok(())
}
