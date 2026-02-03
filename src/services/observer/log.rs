//! Log observer for Phases v2.
//!
//! Handles logging step/workflow execution to files.

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::step::{StepResult, StepState};
use crate::models::workflow::WorkflowState;

/// Log entry for a workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowLogEntry {
    /// Workflow ID (if named)
    pub workflow_id: Option<String>,
    /// Workflow name
    pub workflow_name: String,
    /// Final state
    pub state: WorkflowState,
    /// Number of steps
    pub step_count: usize,
    /// Steps that succeeded
    pub steps_succeeded: usize,
    /// Steps that failed
    pub steps_failed: usize,
    /// Steps that were skipped
    pub steps_skipped: usize,
    /// Total duration in milliseconds
    pub duration_ms: u64,
    /// Start timestamp
    pub started_at: DateTime<Utc>,
    /// End timestamp
    pub ended_at: DateTime<Utc>,
}

/// Log observer for step/workflow execution
pub struct LogObserver {
    /// Base log directory
    log_dir: PathBuf,
    /// Task name
    task: String,
    /// Phase name
    phase: String,
    /// Whether to stream logs in real-time
    stream: bool,
    /// Current log writer
    writer: Option<BufWriter<File>>,
}

impl LogObserver {
    /// Create a new log observer.
    pub fn new(log_dir: impl Into<PathBuf>, task: &str, phase: &str) -> Self {
        Self {
            log_dir: log_dir.into(),
            task: task.to_string(),
            phase: phase.to_string(),
            stream: false,
            writer: None,
        }
    }

    /// Enable real-time streaming.
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    /// Get the log directory for current task/phase.
    pub fn phase_log_dir(&self) -> PathBuf {
        self.log_dir.join(&self.task).join(&self.phase)
    }

    /// Get the log file path for a step.
    pub fn step_log_path(&self, step_index: usize, step_id: Option<&str>) -> PathBuf {
        let filename = if let Some(id) = step_id {
            format!("step-{}-{}.log", step_index, id)
        } else {
            format!("step-{}.log", step_index)
        };
        self.phase_log_dir().join(filename)
    }

    /// Get the context file path (stores workflow metadata).
    pub fn context_path(&self) -> PathBuf {
        self.phase_log_dir().join("context.json")
    }

    /// Initialize log directory.
    pub fn init(&mut self) -> io::Result<()> {
        let dir = self.phase_log_dir();
        fs::create_dir_all(&dir)?;
        Ok(())
    }

    /// Called when a step starts.
    pub fn on_step_start(&mut self, step_index: usize, step_id: Option<&str>) -> io::Result<()> {
        let path = self.step_log_path(step_index, step_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?;

        self.writer = Some(BufWriter::new(file));

        // Write header
        if let Some(ref mut writer) = self.writer {
            writeln!(writer, "# Step {} started at {}", step_index, Utc::now())?;
            writeln!(writer, "# Step ID: {:?}", step_id)?;
            writeln!(writer, "---")?;
            writer.flush()?;
        }

        Ok(())
    }

    /// Called when a step completes.
    pub fn on_step_complete(&mut self, result: &StepResult) -> io::Result<()> {
        if let Some(ref mut writer) = self.writer {
            writeln!(writer, "---")?;
            writeln!(writer, "# Step completed at {}", Utc::now())?;
            writeln!(writer, "# State: {:?}", result.state)?;
            if let Some(code) = result.exit_code {
                writeln!(writer, "# Exit code: {}", code)?;
            }
            if let Some(ref msg) = result.message {
                writeln!(writer, "# Message: {}", msg)?;
            }
            writeln!(writer, "# Duration: {}ms", result.duration_ms)?;
            writer.flush()?;
        }
        self.writer = None;
        Ok(())
    }

    /// Save workflow context/summary.
    pub fn save_workflow_context(&self, entry: &WorkflowLogEntry) -> io::Result<()> {
        let path = self.context_path();
        let json = serde_json::to_string_pretty(entry)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        fs::write(path, json)
    }
}

/// Create workflow log entry from execution results.
pub fn create_workflow_log_entry(
    workflow_id: Option<&str>,
    workflow_name: &str,
    state: WorkflowState,
    step_results: &[StepResult],
    duration_ms: u64,
    started_at: DateTime<Utc>,
) -> WorkflowLogEntry {
    let steps_succeeded = step_results.iter().filter(|r| r.state == StepState::Success).count();
    let steps_failed = step_results.iter().filter(|r| r.state == StepState::Failed).count();
    let steps_skipped = step_results.iter().filter(|r| r.state == StepState::Skipped).count();

    WorkflowLogEntry {
        workflow_id: workflow_id.map(String::from),
        workflow_name: workflow_name.to_string(),
        state,
        step_count: step_results.len(),
        steps_succeeded,
        steps_failed,
        steps_skipped,
        duration_ms,
        started_at,
        ended_at: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_observer_paths() {
        let observer = LogObserver::new("/logs", "auth", "developing");
        assert_eq!(
            observer.phase_log_dir(),
            PathBuf::from("/logs/auth/developing")
        );
        assert_eq!(
            observer.step_log_path(0, None),
            PathBuf::from("/logs/auth/developing/step-0.log")
        );
        assert_eq!(
            observer.step_log_path(1, Some("build")),
            PathBuf::from("/logs/auth/developing/step-1-build.log")
        );
    }

}
