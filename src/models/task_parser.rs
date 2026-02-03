//! Task file parsing utilities.
//!
//! Parses task markdown files with YAML frontmatter.

use std::fs;
use std::path::Path;

use crate::error::{Result, WtError};
use crate::models::{Task, TaskFrontmatter};

/// Parse a task file from disk.
pub fn parse_file(path: &Path) -> Result<Task> {
    let content = fs::read_to_string(path).map_err(|e| WtError::Io {
        operation: "read task file".to_string(),
        path: path.to_string_lossy().to_string(),
        message: e.to_string(),
    })?;
    parse_markdown(&content, path.to_string_lossy().to_string())
}

/// Parse markdown with YAML frontmatter.
///
/// Format:
/// ```markdown
/// ---
/// name: task-name
/// depends:
///   - dep1
///   - dep2
/// ---
///
/// Task description...
/// ```
pub fn parse_markdown(content: &str, file_path: String) -> Result<Task> {
    let content = content.trim();
    if !content.starts_with("---") {
        return Err(WtError::InvalidTaskFile(
            "Missing frontmatter (must start with ---)".to_string(),
        ));
    }

    let rest = &content[3..];
    let end = rest
        .find("---")
        .ok_or_else(|| WtError::InvalidTaskFile("Missing frontmatter end (---)".to_string()))?;

    let yaml = &rest[..end];
    let body = rest[end + 3..].trim();

    let frontmatter: TaskFrontmatter = serde_yaml::from_str(yaml)
        .map_err(|e| WtError::InvalidTaskFile(format!("Invalid frontmatter YAML: {}", e)))?;

    Ok(Task {
        frontmatter,
        content: body.to_string(),
        file_path,
    })
}

/// Validate task name for git branch compatibility.
///
/// Valid names:
/// - Non-empty
/// - No path separators (/, \)
/// - No whitespace
/// - No git-invalid chars (~, ^, :, ?, *, [, @, {)
/// - Cannot start with - or .
/// - Cannot end with . or .lock
/// - Cannot contain ..
pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(WtError::InvalidTaskFile("name cannot be empty".to_string()));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(WtError::InvalidTaskFile(
            "name cannot contain path separators".to_string(),
        ));
    }
    if name.contains(' ') || name.contains('\t') {
        return Err(WtError::InvalidTaskFile(
            "name cannot contain whitespace (invalid for git branch)".to_string(),
        ));
    }
    let invalid_chars = ['~', '^', ':', '?', '*', '[', '\\', '@', '{'];
    for c in invalid_chars {
        if name.contains(c) {
            return Err(WtError::InvalidTaskFile(format!(
                "name cannot contain '{}' (invalid for git branch)",
                c
            )));
        }
    }
    if name.starts_with('-') || name.starts_with('.') {
        return Err(WtError::InvalidTaskFile(
            "name cannot start with '-' or '.' (invalid for git branch)".to_string(),
        ));
    }
    if name.ends_with('.') || name.ends_with(".lock") {
        return Err(WtError::InvalidTaskFile(
            "name cannot end with '.' or '.lock' (invalid for git branch)".to_string(),
        ));
    }
    if name.contains("..") {
        return Err(WtError::InvalidTaskFile(
            "name cannot contain '..' (invalid for git branch)".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== parse_markdown Tests ====================

    #[test]
    fn test_parse_markdown_simple() {
        let content = "---\nname: test\n---\n\nDescription here";
        let task = parse_markdown(content, "test.md".to_string()).unwrap();

        assert_eq!(task.name(), "test");
        assert_eq!(task.content, "Description here");
    }

    #[test]
    fn test_parse_markdown_with_depends() {
        let content = "---\nname: api\ndepends:\n  - auth\n  - db\n---\n\nBuild API";
        let task = parse_markdown(content, "api.md".to_string()).unwrap();

        assert_eq!(task.name(), "api");
        assert_eq!(task.depends(), &["auth".to_string(), "db".to_string()]);
    }

    #[test]
    fn test_parse_markdown_multiline_content() {
        let content = "---\nname: test\n---\n\nLine 1\n\nLine 2\n\n- bullet";
        let task = parse_markdown(content, "test.md".to_string()).unwrap();

        assert_eq!(task.content, "Line 1\n\nLine 2\n\n- bullet");
    }

    #[test]
    fn test_parse_markdown_empty_content() {
        let content = "---\nname: test\n---\n";
        let task = parse_markdown(content, "test.md".to_string()).unwrap();

        assert_eq!(task.content, "");
    }

    #[test]
    fn test_parse_markdown_unicode() {
        let content = "---\nname: 任务\n---\n\n中文描述 🚀";
        let task = parse_markdown(content, "test.md".to_string()).unwrap();

        assert_eq!(task.name(), "任务");
        assert_eq!(task.content, "中文描述 🚀");
    }

    #[test]
    fn test_parse_markdown_missing_frontmatter_start() {
        let content = "name: test\n---\nContent";
        let result = parse_markdown(content, "test.md".to_string());

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Missing frontmatter"));
    }

    #[test]
    fn test_parse_markdown_missing_frontmatter_end() {
        let content = "---\nname: test\nContent without end";
        let result = parse_markdown(content, "test.md".to_string());

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Missing frontmatter end"));
    }

    #[test]
    fn test_parse_markdown_invalid_yaml() {
        let content = "---\nname: [invalid yaml\n---\nContent";
        let result = parse_markdown(content, "test.md".to_string());

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid frontmatter YAML"));
    }

    #[test]
    fn test_parse_markdown_missing_name() {
        let content = "---\ndepends: []\n---\nContent";
        let result = parse_markdown(content, "test.md".to_string());

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_markdown_whitespace_trimmed() {
        let content = "  \n---\nname: test\n---\n\nContent  \n  ";
        let task = parse_markdown(content, "test.md".to_string()).unwrap();

        assert_eq!(task.name(), "test");
    }

    // ==================== validate_name Tests ====================

    #[test]
    fn test_validate_name_valid() {
        assert!(validate_name("auth").is_ok());
        assert!(validate_name("my-task").is_ok());
        assert!(validate_name("task_123").is_ok());
        assert!(validate_name("CamelCase").is_ok());
        assert!(validate_name("a").is_ok());
        assert!(validate_name("task.name").is_ok()); // single dot ok
    }

    #[test]
    fn test_validate_name_empty() {
        let result = validate_name("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_name_path_separators() {
        assert!(validate_name("path/to").is_err());
        assert!(validate_name("path\\to").is_err());
    }

    #[test]
    fn test_validate_name_whitespace() {
        assert!(validate_name("has space").is_err());
        assert!(validate_name("has\ttab").is_err());
        assert!(validate_name(" leading").is_err());
        assert!(validate_name("trailing ").is_err());
    }

    #[test]
    fn test_validate_name_git_invalid_chars() {
        let invalid = ['~', '^', ':', '?', '*', '[', '@', '{'];
        for c in invalid {
            let name = format!("task{}name", c);
            let result = validate_name(&name);
            assert!(result.is_err(), "Should reject char: {}", c);
        }
    }

    #[test]
    fn test_validate_name_invalid_start() {
        assert!(validate_name("-dash").is_err());
        assert!(validate_name(".hidden").is_err());
    }

    #[test]
    fn test_validate_name_invalid_end() {
        assert!(validate_name("name.").is_err());
        assert!(validate_name("name.lock").is_err());
    }

    #[test]
    fn test_validate_name_double_dot() {
        assert!(validate_name("a..b").is_err());
        assert!(validate_name("..").is_err());
    }
}
