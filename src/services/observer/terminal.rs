//! Terminal observer for Phases v2.
//!
//! Handles terminal output for workflow progress display.

use crate::models::step::StepState;
use crate::models::workflow::WorkflowState;

/// Terminal observation settings
#[derive(Debug, Clone, Default)]
pub struct TerminalSettings {
    /// Whether to show progress
    pub show_progress: bool,
}

/// Terminal observer for step/workflow execution
pub struct TerminalObserver {
    settings: TerminalSettings,
}

impl TerminalObserver {
    /// Create a new terminal observer.
    pub fn new(settings: TerminalSettings) -> Self {
        Self { settings }
    }

    /// Called when a step starts.
    pub fn on_step_start(&self, step_index: usize, step_name: &str) {
        if self.settings.show_progress {
            eprintln!("  {} Step {}: {}", "●", step_index + 1, step_name);
        }
    }

    /// Called when a step completes.
    pub fn on_step_complete(&self, step_index: usize, step_name: &str, state: &StepState) {
        if self.settings.show_progress {
            let icon = state.icon();
            eprintln!("  {} Step {}: {} - {:?}", icon, step_index + 1, step_name, state);
        }
    }

    /// Called when a workflow starts.
    pub fn on_workflow_start(&self, workflow_name: &str, step_count: usize) {
        if self.settings.show_progress {
            eprintln!("▶ Starting workflow: {} ({} steps)", workflow_name, step_count);
        }
    }

    /// Called when a workflow completes.
    pub fn on_workflow_complete(&self, workflow_name: &str, state: &WorkflowState, duration_ms: u64) {
        if self.settings.show_progress {
            let icon = state.icon();
            let duration_str = format_duration(duration_ms);
            eprintln!("{} Workflow {} completed: {:?} ({})", icon, workflow_name, state, duration_str);
        }
    }

    /// Called when a step is about to be retried.
    pub fn on_step_retry(&self, step_index: usize, step_name: &str, attempt: u32, max_attempts: u32, delay_ms: u64) {
        if self.settings.show_progress {
            let delay_str = format_duration(delay_ms);
            eprintln!(
                "  ↻ Step {}: {} - Retry {}/{} in {}",
                step_index + 1, step_name, attempt + 1, max_attempts, delay_str
            );
        }
    }
}

/// Format duration in human-readable form.
fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else if ms < 3_600_000 {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) / 1000;
        format!("{}m {}s", mins, secs)
    } else {
        let hours = ms / 3_600_000;
        let mins = (ms % 3_600_000) / 60_000;
        format!("{}h {}m", hours, mins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(500), "500ms");
        assert_eq!(format_duration(1500), "1.5s");
        assert_eq!(format_duration(65000), "1m 5s");
        assert_eq!(format_duration(3665000), "1h 1m");
    }

    #[test]
    fn test_terminal_observer_creation() {
        let settings = TerminalSettings {
            show_progress: true,
        };
        let observer = TerminalObserver::new(settings);
        assert!(observer.settings.show_progress);
    }
}
