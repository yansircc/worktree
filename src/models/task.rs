//! Task definitions and data structures.

use serde::{Deserialize, Serialize};

use crate::services::multiplexer::MultiplexerType;

/// Runtime instance information for a running task
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
    /// Task description (markdown content after frontmatter)
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
    fn test_task_frontmatter_serialize_minimal() {
        let fm = TaskFrontmatter {
            name: "test".to_string(),
            depends: vec![],
        };
        let yaml = serde_yaml::to_string(&fm).unwrap();

        assert!(yaml.contains("name: test"));
        assert!(!yaml.contains("depends:"));
    }

    #[test]
    fn test_task_frontmatter_deserialize_minimal() {
        let yaml = "name: test\n";
        let fm: TaskFrontmatter = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(fm.name, "test");
        assert!(fm.depends.is_empty());
    }

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
