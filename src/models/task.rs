use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    #[default]
    Pending,
    Running,
    Review,
    Completed,
}

impl TaskStatus {
    /// Check if transition to target status is valid.
    ///
    /// Valid transitions:
    /// - Pending -> Running
    /// - Running -> Review
    /// - Review -> Running (resume)
    /// - Review -> Completed
    pub fn can_transition_to(&self, target: &TaskStatus) -> bool {
        matches!(
            (self, target),
            (TaskStatus::Pending, TaskStatus::Running)
                | (TaskStatus::Running, TaskStatus::Review)
                | (TaskStatus::Review, TaskStatus::Running)
                | (TaskStatus::Review, TaskStatus::Completed)
        )
    }

    /// Get display name for the status.
    pub fn display_name(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Review => "review",
            TaskStatus::Completed => "completed",
        }
    }

    /// Get plain status icon (without color).
    pub fn icon(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "○",
            TaskStatus::Running => "●",
            TaskStatus::Review => "?",
            TaskStatus::Completed => "✓",
        }
    }

    /// Get colored status icon for terminal display.
    pub fn colored_icon(&self) -> String {
        use crate::display::{GREEN, MAGENTA, RESET, WHITE, YELLOW};

        let color = match self {
            TaskStatus::Pending => WHITE,
            TaskStatus::Running => GREEN,
            TaskStatus::Review => YELLOW,
            TaskStatus::Completed => MAGENTA,
        };
        format!("{}{}{}", color, self.icon(), RESET)
    }
}

use crate::services::multiplexer::MultiplexerType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub branch: String,
    pub worktree_path: String,
    pub session_name: String,
    pub window_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default = "default_multiplexer")]
    pub multiplexer: MultiplexerType,
}

fn default_multiplexer() -> MultiplexerType {
    MultiplexerType::Tmux
}

impl Instance {
    /// Get the multiplexer type for this instance
    pub fn multiplexer_type(&self) -> MultiplexerType {
        self.multiplexer
    }
}

/// Frontmatter of task markdown file (definition only, no runtime state)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFrontmatter {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends: Vec<String>,
}

/// Full task with frontmatter and content
#[derive(Debug, Clone)]
pub struct Task {
    pub frontmatter: TaskFrontmatter,
    #[allow(dead_code)]
    pub content: String,
    pub file_path: String,
}

impl Task {
    pub fn name(&self) -> &str {
        &self.frontmatter.name
    }

    pub fn depends(&self) -> &[String] {
        &self.frontmatter.depends
    }
}

/// Input for creating a task via JSON
#[derive(Debug, Deserialize)]
pub struct TaskInput {
    pub name: String,
    #[serde(default)]
    pub depends: Vec<String>,
    pub description: String,
}

impl TaskInput {
    pub fn to_markdown(&self) -> String {
        let frontmatter = TaskFrontmatter {
            name: self.name.clone(),
            depends: self.depends.clone(),
        };
        format_task_markdown(&frontmatter, &self.description)
    }
}

/// Format task as markdown with YAML frontmatter.
pub fn format_task_markdown(frontmatter: &TaskFrontmatter, content: &str) -> String {
    let yaml = serde_yaml::to_string(frontmatter).unwrap_or_default();
    format!("---\n{}---\n\n{}\n", yaml, content)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== TaskStatus Tests ====================

    #[test]
    fn test_task_status_default() {
        let status: TaskStatus = Default::default();
        assert_eq!(status, TaskStatus::Pending);
    }

    #[test]
    fn test_task_status_can_transition_to() {
        // Valid transitions
        assert!(TaskStatus::Pending.can_transition_to(&TaskStatus::Running));
        assert!(TaskStatus::Running.can_transition_to(&TaskStatus::Review));
        assert!(TaskStatus::Review.can_transition_to(&TaskStatus::Running)); // resume
        assert!(TaskStatus::Review.can_transition_to(&TaskStatus::Completed));

        // Invalid transitions
        assert!(!TaskStatus::Pending.can_transition_to(&TaskStatus::Review));
        assert!(!TaskStatus::Pending.can_transition_to(&TaskStatus::Completed));
        assert!(!TaskStatus::Running.can_transition_to(&TaskStatus::Completed));
        assert!(!TaskStatus::Completed.can_transition_to(&TaskStatus::Pending));
        assert!(!TaskStatus::Completed.can_transition_to(&TaskStatus::Running));
    }

    #[test]
    fn test_task_status_display_name() {
        assert_eq!(TaskStatus::Pending.display_name(), "pending");
        assert_eq!(TaskStatus::Running.display_name(), "running");
        assert_eq!(TaskStatus::Review.display_name(), "review");
        assert_eq!(TaskStatus::Completed.display_name(), "completed");
    }

    #[test]
    fn test_task_status_serialize() {
        assert_eq!(
            serde_yaml::to_string(&TaskStatus::Pending).unwrap().trim(),
            "pending"
        );
        assert_eq!(
            serde_yaml::to_string(&TaskStatus::Running).unwrap().trim(),
            "running"
        );
        assert_eq!(
            serde_yaml::to_string(&TaskStatus::Review).unwrap().trim(),
            "review"
        );
        assert_eq!(
            serde_yaml::to_string(&TaskStatus::Completed).unwrap().trim(),
            "completed"
        );
    }

    #[test]
    fn test_task_status_deserialize() {
        assert_eq!(
            serde_yaml::from_str::<TaskStatus>("pending").unwrap(),
            TaskStatus::Pending
        );
        assert_eq!(
            serde_yaml::from_str::<TaskStatus>("running").unwrap(),
            TaskStatus::Running
        );
        assert_eq!(
            serde_yaml::from_str::<TaskStatus>("review").unwrap(),
            TaskStatus::Review
        );
        assert_eq!(
            serde_yaml::from_str::<TaskStatus>("completed").unwrap(),
            TaskStatus::Completed
        );
    }

    #[test]
    fn test_task_status_deserialize_invalid() {
        let result = serde_yaml::from_str::<TaskStatus>("invalid");
        assert!(result.is_err());
    }

    // ==================== TaskInput Tests ====================

    #[test]
    fn test_task_input_to_markdown_simple() {
        let input = TaskInput {
            name: "auth".to_string(),
            depends: vec![],
            description: "Implement authentication".to_string(),
        };
        let md = input.to_markdown();

        assert!(md.starts_with("---\n"));
        assert!(md.contains("name: auth"));
        // No status field anymore
        assert!(!md.contains("status:"));
        assert!(md.ends_with("Implement authentication\n"));
    }

    #[test]
    fn test_task_input_to_markdown_with_depends() {
        let input = TaskInput {
            name: "api".to_string(),
            depends: vec!["auth".to_string(), "database".to_string()],
            description: "Build API".to_string(),
        };
        let md = input.to_markdown();

        assert!(md.contains("name: api"));
        assert!(md.contains("depends:"));
        assert!(md.contains("- auth"));
        assert!(md.contains("- database"));
    }

    #[test]
    fn test_task_input_to_markdown_multiline_description() {
        let input = TaskInput {
            name: "feature".to_string(),
            depends: vec![],
            description: "Line 1\n\nLine 2\n- bullet".to_string(),
        };
        let md = input.to_markdown();

        assert!(md.contains("Line 1\n\nLine 2\n- bullet"));
    }

    #[test]
    fn test_task_input_to_markdown_unicode() {
        let input = TaskInput {
            name: "unicode".to_string(),
            depends: vec![],
            description: "实现用户认证 🔐".to_string(),
        };
        let md = input.to_markdown();

        assert!(md.contains("实现用户认证 🔐"));
    }

    #[test]
    fn test_task_input_deserialize() {
        let json = r#"{"name": "test", "depends": ["a", "b"], "description": "desc"}"#;
        let input: TaskInput = serde_json::from_str(json).unwrap();

        assert_eq!(input.name, "test");
        assert_eq!(input.depends, vec!["a", "b"]);
        assert_eq!(input.description, "desc");
    }

    #[test]
    fn test_task_input_deserialize_empty_depends() {
        let json = r#"{"name": "test", "description": "desc"}"#;
        let input: TaskInput = serde_json::from_str(json).unwrap();

        assert_eq!(input.name, "test");
        assert!(input.depends.is_empty());
    }

    #[test]
    fn test_task_input_deserialize_missing_name() {
        let json = r#"{"depends": [], "description": "desc"}"#;
        let result = serde_json::from_str::<TaskInput>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_task_input_deserialize_missing_description() {
        let json = r#"{"name": "test", "depends": []}"#;
        let result = serde_json::from_str::<TaskInput>(json);
        assert!(result.is_err());
    }

    // ==================== TaskFrontmatter Tests ====================

    #[test]
    fn test_task_frontmatter_serialize_minimal() {
        let fm = TaskFrontmatter {
            name: "test".to_string(),
            depends: vec![],
        };
        let yaml = serde_yaml::to_string(&fm).unwrap();

        assert!(yaml.contains("name: test"));
        // No status or instance fields
        assert!(!yaml.contains("status:"));
        assert!(!yaml.contains("instance:"));
        // Empty depends should be skipped
        assert!(!yaml.contains("depends:"));
    }

    #[test]
    fn test_task_frontmatter_serialize_with_depends() {
        let fm = TaskFrontmatter {
            name: "test".to_string(),
            depends: vec!["dep1".to_string()],
        };
        let yaml = serde_yaml::to_string(&fm).unwrap();

        assert!(yaml.contains("name: test"));
        assert!(yaml.contains("depends:"));
        assert!(yaml.contains("- dep1"));
    }

    #[test]
    fn test_task_frontmatter_deserialize_minimal() {
        let yaml = "name: test\n";
        let fm: TaskFrontmatter = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(fm.name, "test");
        assert!(fm.depends.is_empty());
    }

    // ==================== Task Tests ====================

    #[test]
    fn test_task_accessors() {
        let task = Task {
            frontmatter: TaskFrontmatter {
                name: "myname".to_string(),
                depends: vec!["dep1".to_string()],
            },
            content: "content".to_string(),
            file_path: "path".to_string(),
        };

        assert_eq!(task.name(), "myname");
        assert_eq!(task.depends(), &["dep1".to_string()]);
    }
}
