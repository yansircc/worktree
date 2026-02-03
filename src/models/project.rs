//! Project model for Phases v2 system.
//!
//! Project is the top-level concept in wt, containing:
//! - All tasks and their states
//! - Global configuration (phases, workflows, concurrency)
//! - Resource settings (multiplexer, worktree directory)

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::phase::Phase;
use super::workflow::Workflow;

// ============================================================================
// ProjectStatus
// ============================================================================

/// Aggregated project status (derived from task statuses)
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProjectStatus {
    /// Total number of tasks
    pub total: usize,
    /// Tasks in pending state
    pub pending: usize,
    /// Tasks in active state
    pub active: usize,
    /// Tasks in idle state
    pub idle: usize,
    /// Tasks completed
    pub completed: usize,
    /// Completion progress (0.0 - 1.0)
    pub progress: f32,
}

impl ProjectStatus {
    /// Create a new project status from task counts
    pub fn new(pending: usize, active: usize, idle: usize, completed: usize) -> Self {
        let total = pending + active + idle + completed;
        let progress = if total > 0 {
            completed as f32 / total as f32
        } else {
            0.0
        };
        Self {
            total,
            pending,
            active,
            idle,
            completed,
            progress,
        }
    }

    /// Check if all tasks are completed
    pub fn is_all_completed(&self) -> bool {
        self.total > 0 && self.completed == self.total
    }

    /// Check if any task is blocked (idle)
    pub fn has_blocked(&self) -> bool {
        self.idle > 0
    }

    /// Get progress as percentage string
    pub fn progress_percent(&self) -> String {
        format!("{:.0}%", self.progress * 100.0)
    }

    /// Get summary string
    pub fn summary(&self) -> String {
        format!(
            "{}/{} completed ({} active, {} idle, {} pending)",
            self.completed, self.total, self.active, self.idle, self.pending
        )
    }
}

// ============================================================================
// Resource Config
// ============================================================================

/// Multiplexer type
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MultiplexerType {
    /// tmux (default)
    #[default]
    Tmux,
    /// Zellij
    Zellij,
}

/// Project resource configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    /// Multiplexer type
    #[serde(default)]
    pub multiplexer: MultiplexerType,
    /// Session name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// Worktree directory
    #[serde(default = "default_worktree_dir")]
    pub worktree_dir: String,
}

fn default_worktree_dir() -> String {
    ".wt/worktrees".to_string()
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            multiplexer: MultiplexerType::default(),
            session: None,
            worktree_dir: default_worktree_dir(),
        }
    }
}

// ============================================================================
// Concurrency Config
// ============================================================================

/// Resource limits for concurrency
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// CPU limit (e.g., "80%")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu: Option<String>,
    /// Memory limit (e.g., "8GB")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<String>,
}

/// Project concurrency configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrencyConfig {
    /// Maximum concurrent active tasks
    #[serde(default = "default_max_active_tasks")]
    pub max_active_tasks: usize,
    /// Maximum concurrent agents
    #[serde(default = "default_max_agents")]
    pub max_agents: usize,
    /// Resource limits
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_limits: Option<ResourceLimits>,
}

fn default_max_active_tasks() -> usize {
    5
}

fn default_max_agents() -> usize {
    3
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            max_active_tasks: default_max_active_tasks(),
            max_agents: default_max_agents(),
            resource_limits: None,
        }
    }
}

// ============================================================================
// Phases Config
// ============================================================================

/// Project phases configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhasesConfig {
    /// Phase sequence (e.g., ["pending", "developing", "reviewing", "completed"])
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sequence: Vec<String>,
    /// Phase definitions (override defaults)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub definitions: HashMap<String, Phase>,
}

impl PhasesConfig {
    /// Get phase sequence or default
    pub fn sequence_or_default(&self) -> Vec<String> {
        if self.sequence.is_empty() {
            super::phase::DEFAULT_PHASE_SEQUENCE
                .iter()
                .map(|s| s.to_string())
                .collect()
        } else {
            self.sequence.clone()
        }
    }
}

// ============================================================================
// Notification Config
// ============================================================================

/// Notification backend
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NotifyBackend {
    /// No notifications
    #[default]
    None,
    /// Slack
    Slack,
    /// Email
    Email,
    /// System notification
    System,
}

/// Project notification configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Notification backend
    #[serde(default)]
    pub backend: NotifyBackend,
    /// Notify when all tasks completed
    #[serde(default)]
    pub on_all_completed: bool,
    /// Notify when any task is blocked
    #[serde(default)]
    pub on_any_blocked: bool,
}

// ============================================================================
// Observe Config
// ============================================================================

/// Project observation configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectObserve {
    /// Enable dashboard
    #[serde(default)]
    pub dashboard: bool,
    /// Notification configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notifications: Option<NotificationConfig>,
}

// ============================================================================
// Project
// ============================================================================

/// Project configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Project name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Project description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Resource configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceConfig>,

    /// Phases configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phases: Option<PhasesConfig>,

    /// Workflow library (reusable workflow fragments)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub workflows: HashMap<String, Workflow>,

    /// Concurrency configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<ConcurrencyConfig>,

    /// Observation configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observe: Option<ProjectObserve>,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            name: None,
            description: None,
            resources: None,
            phases: None,
            workflows: HashMap::new(),
            concurrency: None,
            observe: None,
        }
    }
}

impl Project {
    /// Create a new project with a name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            ..Default::default()
        }
    }

    /// Get project name or default
    pub fn name_or_default(&self) -> &str {
        self.name.as_deref().unwrap_or("wt")
    }

    /// Get multiplexer type
    pub fn multiplexer(&self) -> MultiplexerType {
        self.resources
            .as_ref()
            .map(|r| r.multiplexer.clone())
            .unwrap_or_default()
    }

    /// Get session name
    pub fn session_name(&self) -> String {
        self.resources
            .as_ref()
            .and_then(|r| r.session.clone())
            .unwrap_or_else(|| self.name_or_default().to_string())
    }

    /// Get worktree directory
    pub fn worktree_dir(&self) -> &str {
        self.resources
            .as_ref()
            .map(|r| r.worktree_dir.as_str())
            .unwrap_or(".wt/worktrees")
    }

    /// Get phase sequence
    pub fn phase_sequence(&self) -> Vec<String> {
        self.phases
            .as_ref()
            .map(|p| p.sequence_or_default())
            .unwrap_or_else(|| {
                super::phase::DEFAULT_PHASE_SEQUENCE
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            })
    }

    /// Get max concurrent active tasks
    pub fn max_active_tasks(&self) -> usize {
        self.concurrency
            .as_ref()
            .map(|c| c.max_active_tasks)
            .unwrap_or_else(default_max_active_tasks)
    }

    /// Get max concurrent agents
    pub fn max_agents(&self) -> usize {
        self.concurrency
            .as_ref()
            .map(|c| c.max_agents)
            .unwrap_or_else(default_max_agents)
    }

    /// Get a workflow by name
    pub fn get_workflow(&self, name: &str) -> Option<&Workflow> {
        self.workflows.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_status_new() {
        let status = ProjectStatus::new(2, 1, 1, 1);
        assert_eq!(status.total, 5);
        assert_eq!(status.pending, 2);
        assert_eq!(status.active, 1);
        assert_eq!(status.idle, 1);
        assert_eq!(status.completed, 1);
        assert!((status.progress - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_project_status_empty() {
        let status = ProjectStatus::new(0, 0, 0, 0);
        assert_eq!(status.total, 0);
        assert_eq!(status.progress, 0.0);
        assert!(!status.is_all_completed());
    }

    #[test]
    fn test_project_status_all_completed() {
        let status = ProjectStatus::new(0, 0, 0, 5);
        assert!(status.is_all_completed());
        assert_eq!(status.progress, 1.0);
    }

    #[test]
    fn test_project_status_summary() {
        let status = ProjectStatus::new(1, 2, 1, 1);
        let summary = status.summary();
        assert!(summary.contains("1/5 completed"));
        assert!(summary.contains("2 active"));
    }

    #[test]
    fn test_project_defaults() {
        let project = Project::default();
        assert_eq!(project.name_or_default(), "wt");
        assert_eq!(project.session_name(), "wt");
        assert_eq!(project.worktree_dir(), ".wt/worktrees");
        assert_eq!(project.max_active_tasks(), 5);
        assert_eq!(project.max_agents(), 3);
    }

    #[test]
    fn test_project_phase_sequence() {
        let project = Project::default();
        let seq = project.phase_sequence();
        assert_eq!(seq, vec!["pending", "developing", "reviewing", "completed"]);
    }

    #[test]
    fn test_project_serialize() {
        let project = Project::new("my-project");
        let json = serde_json::to_string(&project).unwrap();
        assert!(json.contains("my-project"));

        let parsed: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, Some("my-project".to_string()));
    }

    #[test]
    fn test_multiplexer_type() {
        let project = Project {
            resources: Some(ResourceConfig {
                multiplexer: MultiplexerType::Zellij,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(project.multiplexer(), MultiplexerType::Zellij);
    }
}
