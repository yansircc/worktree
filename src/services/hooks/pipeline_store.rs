//! Pipeline status storage and management.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::error::{Result, WtError};

use super::pipeline::{PipelineState, PipelineStatus};

/// Manages pipeline status files in `.wt/pipelines/`
pub struct PipelineStore {
    dir: PathBuf,
}

impl PipelineStore {
    /// Create a store for the given repo root
    pub fn new(repo_root: &str) -> Self {
        Self {
            dir: PathBuf::from(repo_root).join(".wt/pipelines"),
        }
    }

    /// Ensure the pipelines directory exists
    pub fn ensure_dir(&self) -> Result<()> {
        if !self.dir.exists() {
            fs::create_dir_all(&self.dir).map_err(|e| WtError::Io {
                operation: "create_dir".to_string(),
                path: self.dir.display().to_string(),
                message: e.to_string(),
            })?;
        }
        Ok(())
    }

    /// Get the status file path for a pipeline
    pub fn status_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{}.json", id))
    }

    /// Get the log file path for a pipeline
    pub fn log_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{}.log", id))
    }

    /// Load a pipeline status by ID
    pub fn load(&self, id: &str) -> Result<PipelineStatus> {
        let path = self.status_path(id);
        if !path.exists() {
            return Err(WtError::InvalidInput(format!(
                "Pipeline '{}' not found",
                id
            )));
        }

        let content = fs::read_to_string(&path).map_err(|e| WtError::Io {
            operation: "read".to_string(),
            path: path.display().to_string(),
            message: e.to_string(),
        })?;

        serde_json::from_str(&content).map_err(|e| WtError::ConfigRead(e.to_string()))
    }

    /// Save a pipeline status
    pub fn save(&self, status: &PipelineStatus) -> Result<()> {
        self.ensure_dir()?;
        let path = self.status_path(&status.id);
        let json =
            serde_json::to_string_pretty(status).map_err(|e| WtError::ConfigRead(e.to_string()))?;

        fs::write(&path, json).map_err(|e| WtError::Io {
            operation: "write".to_string(),
            path: path.display().to_string(),
            message: e.to_string(),
        })
    }

    /// List all pipelines (with status refresh)
    pub fn list(&self) -> Result<Vec<PipelineStatus>> {
        if !self.dir.exists() {
            return Ok(Vec::new());
        }

        let mut pipelines = Vec::new();
        for path in self.iter_json_files()? {
            if let Ok(mut status) = self.load_from_path(&path) {
                self.refresh_status(&mut status, &path);
                pipelines.push(status);
            }
        }

        pipelines.sort_by(|a, b| b.start_time.cmp(&a.start_time));
        Ok(pipelines)
    }

    /// Remove a pipeline and its associated files
    pub fn remove(&self, id: &str) {
        let _ = fs::remove_file(self.status_path(id));
        let _ = fs::remove_file(self.log_path(id));
        let _ = fs::remove_file(self.dir.join(format!("{}.log.exit", id)));
    }

    // === Private helpers ===

    fn iter_json_files(&self) -> Result<Vec<PathBuf>> {
        let entries = fs::read_dir(&self.dir).map_err(|e| WtError::Io {
            operation: "read_dir".to_string(),
            path: self.dir.display().to_string(),
            message: e.to_string(),
        })?;

        Ok(entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map_or(false, |ext| ext == "json"))
            .collect())
    }

    fn load_from_path(&self, path: &PathBuf) -> Result<PipelineStatus> {
        let content = fs::read_to_string(path).map_err(|e| WtError::Io {
            operation: "read".to_string(),
            path: path.display().to_string(),
            message: e.to_string(),
        })?;
        serde_json::from_str(&content).map_err(|e| WtError::ConfigRead(e.to_string()))
    }

    fn refresh_status(&self, status: &mut PipelineStatus, path: &PathBuf) {
        if status.status != PipelineState::Running {
            return;
        }

        // Check exit file or process status
        let exit_file = path.with_extension("log.exit");
        if exit_file.exists() {
            let exit_code = fs::read_to_string(&exit_file)
                .unwrap_or_default()
                .trim()
                .parse::<i32>()
                .unwrap_or(-1);
            status.status = if exit_code == 0 {
                PipelineState::Completed
            } else {
                PipelineState::Failed
            };
            let _ = self.save(status);
        } else if let Some(pid) = status.pid {
            // Check if process exists
            let check = Command::new("kill")
                .args(["-0", &pid.to_string()])
                .output();
            if check.map_or(true, |o| !o.status.success()) {
                status.status = PipelineState::Failed;
                let _ = self.save(status);
            }
        }
    }
}
