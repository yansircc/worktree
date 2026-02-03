// ==================== File-based Task Tests ====================

#[test]
fn test_task_file_roundtrip() {
    let content = r#"---
name: roundtrip
depends:
  - dep1
---

Task description with **markdown**

- bullet 1
- bullet 2
"#;

    let task = wt::models::task_parser::parse_markdown(content, "roundtrip.md".to_string()).unwrap();
    assert_eq!(task.name(), "roundtrip");
    assert_eq!(task.depends(), &["dep1".to_string()]);
    assert!(task.content.contains("**markdown**"));
    assert!(task.content.contains("- bullet 1"));
}

#[test]
fn test_parse_task_file_with_code_blocks() {
    let content = r#"---
name: codeblock
---

Here's some code:

```rust
fn main() {
    println!("Hello");
}
```

And more text.
"#;

    let task = wt::models::task_parser::parse_markdown(content, "codeblock.md".to_string()).unwrap();
    assert!(task.content.contains("```rust"));
    assert!(task.content.contains("fn main()"));
}

#[test]
fn test_parse_task_file_with_frontmatter_in_content() {
    let content = r#"---
name: tricky
---

Some text

---

This is a horizontal rule, not frontmatter.

---

Another one.
"#;

    let task = wt::models::task_parser::parse_markdown(content, "tricky.md".to_string()).unwrap();
    assert_eq!(task.name(), "tricky");
    assert!(task.content.contains("---"));
}

// ==================== Edge Cases ====================

#[test]
fn test_task_with_very_long_name() {
    let long_name = "a".repeat(200);
    let content = format!("---\nname: {}\n---\n\nContent", long_name);

    let task = wt::models::task_parser::parse_markdown(&content, "long.md".to_string()).unwrap();
    assert_eq!(task.name(), long_name);
}

#[test]
fn test_task_with_special_yaml_chars_in_description() {
    let content = r#"---
name: special
---

Description with: colons
And "quotes"
And 'single quotes'
And {braces}
And [brackets]
"#;

    let task = wt::models::task_parser::parse_markdown(content, "special.md".to_string()).unwrap();
    assert!(task.content.contains("colons"));
    assert!(task.content.contains("\"quotes\""));
}

#[test]
fn test_task_with_empty_depends_array() {
    let content = "---\nname: empty\ndepends: []\n---\n\nContent";

    let task = wt::models::task_parser::parse_markdown(content, "empty.md".to_string()).unwrap();
    assert!(task.depends().is_empty());
}

#[test]
fn test_task_with_many_dependencies() {
    let deps: Vec<String> = (0..50).map(|i| format!("dep{}", i)).collect();
    let deps_yaml = deps
        .iter()
        .map(|d| format!("  - {}", d))
        .collect::<Vec<_>>()
        .join("\n");
    let content = format!("---\nname: many\ndepends:\n{}\n---\n\nContent", deps_yaml);

    let task = wt::models::task_parser::parse_markdown(&content, "many.md".to_string()).unwrap();
    assert_eq!(task.depends().len(), 50);
}

// ==================== TaskInput to Markdown ====================

#[test]
fn test_task_input_roundtrip() {
    use wt::models::TaskInput;

    let input = TaskInput {
        name: "roundtrip".to_string(),
        depends: vec!["a".to_string(), "b".to_string()],
        description: "Test description".to_string(),
    };

    let markdown = input.to_markdown();
    let task = wt::models::task_parser::parse_markdown(&markdown, "test.md".to_string()).unwrap();

    assert_eq!(task.name(), "roundtrip");
    assert_eq!(task.depends(), &["a".to_string(), "b".to_string()]);
    assert_eq!(task.content, "Test description");
}

// ==================== Validation Edge Cases ====================

#[test]
fn test_validate_name_unicode() {
    assert!(wt::models::task_parser::validate_name("任务").is_ok());
    assert!(wt::models::task_parser::validate_name("tâche").is_ok());
    assert!(wt::models::task_parser::validate_name("задача").is_ok());
}

#[test]
fn test_validate_name_numbers() {
    assert!(wt::models::task_parser::validate_name("123").is_ok());
    assert!(wt::models::task_parser::validate_name("task123").is_ok());
    assert!(wt::models::task_parser::validate_name("123task").is_ok());
}

#[test]
fn test_validate_name_underscores_and_hyphens() {
    assert!(wt::models::task_parser::validate_name("my_task").is_ok());
    assert!(wt::models::task_parser::validate_name("my-task").is_ok());
    assert!(wt::models::task_parser::validate_name("my_task-name").is_ok());
    assert!(wt::models::task_parser::validate_name("__double__").is_ok());
    assert!(wt::models::task_parser::validate_name("--double--").is_err());
}

#[test]
fn test_validate_name_edge_cases() {
    assert!(wt::models::task_parser::validate_name("a").is_ok());
    assert!(wt::models::task_parser::validate_name("0").is_ok());
    assert!(wt::models::task_parser::validate_name("task1").is_ok());
    assert!(wt::models::task_parser::validate_name("MyTask").is_ok());
}

// ==================== Config Edge Cases ====================

#[test]
fn test_config_extra_fields_ignored() {
    let jsonc = r#"{
        "multiplexer": "tmux",
        "unknown_field": "should be ignored",
        "another_unknown": "also ignored"
    }"#;
    let result = wt::models::WtConfig::from_str(jsonc);
    assert!(result.is_ok());
}

#[test]
fn test_config_empty_copy_files() {
    let jsonc = r#"{
        "multiplexer": "tmux",
        "copy_files": []
    }"#;
    let config = wt::models::WtConfig::from_str(jsonc).unwrap();
    assert!(config.copy_files.is_empty());
}
