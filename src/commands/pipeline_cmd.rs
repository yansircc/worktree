//! Pipeline management commands.
//!
//! Manage background pipelines: list, logs, kill, cleanup.

use std::fs;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

use crate::cli::PipelineAction;
use crate::error::Result;
use crate::services::hooks::{cleanup_pipelines, kill_pipeline, list_pipelines, PipelineState};

/// Execute pipeline subcommand
pub fn execute(action: PipelineAction) -> Result<()> {
    // Get repo root
    let repo_root = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());

    match action {
        PipelineAction::List { json } => list(&repo_root, json),
        PipelineAction::Logs { id, follow, lines } => logs(&repo_root, &id, follow, lines),
        PipelineAction::Kill { id } => kill(&repo_root, &id),
        PipelineAction::Cleanup { max_age } => cleanup(&repo_root, max_age),
    }
}

/// List all pipelines
fn list(repo_root: &str, json_output: bool) -> Result<()> {
    let pipelines = list_pipelines(repo_root)?;

    if json_output {
        let json = serde_json::to_string_pretty(&pipelines).unwrap_or_default();
        println!("{}", json);
        return Ok(());
    }

    if pipelines.is_empty() {
        println!("No pipelines found.");
        return Ok(());
    }

    println!(
        "{:<30} {:<10} {:<20} {}",
        "ID", "STATUS", "STARTED", "NAME"
    );
    println!("{}", "-".repeat(80));

    for p in pipelines {
        let status_str = match p.status {
            PipelineState::Running => "\x1b[32m●\x1b[0m running",
            PipelineState::Completed => "\x1b[34m✓\x1b[0m completed",
            PipelineState::Failed => "\x1b[31m✗\x1b[0m failed",
            PipelineState::Killed => "\x1b[33m⊘\x1b[0m killed",
        };

        // Parse and format timestamp
        let started = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&p.start_time) {
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            p.start_time.clone()
        };

        let name = p.name.unwrap_or_else(|| "-".to_string());

        println!("{:<30} {:<10} {:<20} {}", p.id, status_str, started, name);
    }

    Ok(())
}

/// Show pipeline logs
fn logs(repo_root: &str, pipeline_id: &str, follow: bool, lines: usize) -> Result<()> {
    let pipelines = list_pipelines(repo_root)?;
    let pipeline = pipelines.iter().find(|p| p.id == pipeline_id);

    let output_file = match pipeline {
        Some(p) => p.output_file.clone(),
        None => {
            // Try to find log file directly
            let log_path = format!("{}/.wt/pipelines/{}.log", repo_root, pipeline_id);
            if std::path::Path::new(&log_path).exists() {
                Some(log_path)
            } else {
                None
            }
        }
    };

    let output_file = output_file.ok_or_else(|| {
        crate::error::WtError::InvalidInput(format!("Pipeline '{}' not found", pipeline_id))
    })?;

    if follow {
        // Use tail -f
        let mut child = Command::new("tail")
            .args(["-f", &output_file])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| crate::error::WtError::Script {
                script: "tail -f".to_string(),
                message: e.to_string(),
            })?;

        let _ = child.wait();
    } else {
        // Read last N lines
        let file = fs::File::open(&output_file).map_err(|e| crate::error::WtError::Io {
            operation: "open".to_string(),
            path: output_file.clone(),
            message: e.to_string(),
        })?;

        let reader = BufReader::new(file);
        let all_lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();

        let start = if all_lines.len() > lines {
            all_lines.len() - lines
        } else {
            0
        };

        for line in &all_lines[start..] {
            println!("{}", line);
        }
    }

    Ok(())
}

/// Kill a pipeline
fn kill(repo_root: &str, pipeline_id: &str) -> Result<()> {
    kill_pipeline(repo_root, pipeline_id)?;
    println!("Pipeline '{}' killed.", pipeline_id);
    Ok(())
}

/// Cleanup old pipelines
fn cleanup(repo_root: &str, max_age_hours: u64) -> Result<()> {
    let removed = cleanup_pipelines(repo_root, max_age_hours)?;

    if removed > 0 {
        println!(
            "Cleaned up {} pipeline record(s) older than {} hours.",
            removed, max_age_hours
        );
    } else {
        println!("No old pipelines to clean up.");
    }

    Ok(())
}
