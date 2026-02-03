//! Built-in predefined pipelines.
//!
//! Provides default pipeline configurations for common workflows.

use crate::models::{AgentStep, Step};

/// Get a built-in predefined pipeline by name.
///
/// Available pipelines:
/// - `code-review`: Quick lint (haiku) → Deep review (sonnet)
/// - `merge`: Rebase and squash merge (sonnet)
/// - `refactor`: Analyze (haiku) → Apply changes (sonnet)
pub fn get(name: &str) -> Option<Vec<Step>> {
    match name {
        "code-review" => Some(code_review_pipeline()),
        "merge" => Some(merge_pipeline()),
        "refactor" => Some(refactor_pipeline()),
        _ => None,
    }
}

/// Code review pipeline: quick check then deep review.
fn code_review_pipeline() -> Vec<Step> {
    vec![
        Step::Agent {
            agent: AgentStep::new("Quick lint check for task ${task}. Report any obvious issues.")
                .with_model("haiku")
                .with_print()
                .with_max_turns(5)
                .with_tools(vec!["Read".into(), "Grep".into(), "Glob".into()])
                .with_no_session_persistence()
                .with_include_partial_messages(),
        },
        Step::Agent {
            agent: AgentStep::new(
                "Deep code review for task ${task}. Check for bugs, security issues, and suggest improvements.",
            )
            .with_model("sonnet")
            .with_print()
            .with_max_turns(10)
            .with_tools(vec!["Read".into(), "Grep".into(), "Glob".into()])
            .with_no_session_persistence()
            .with_include_partial_messages(),
        },
    ]
}

/// Merge pipeline: rebase onto main and squash merge.
fn merge_pipeline() -> Vec<Step> {
    vec![Step::Agent {
        agent: AgentStep::new(
            "Merge task ${task}. Rebase ${branch} onto main, resolve conflicts if any, then squash merge.",
        )
        .with_model("sonnet")
        .with_print()
        .with_max_turns(20)
        .with_tools(vec!["Bash".into(), "Read".into(), "Edit".into()])
        .with_allowed_tools(vec!["Bash(git *)".into()])
        .with_append_system_prompt(
            "You are a git expert. Steps: 1) git fetch origin, 2) git rebase origin/main, 3) resolve conflicts if any, 4) git checkout main, 5) git merge --squash ${branch}, 6) git commit. Report any issues.",
        )
        .with_no_session_persistence()
        .with_include_partial_messages(),
    }]
}

/// Refactor pipeline: analyze then apply changes.
fn refactor_pipeline() -> Vec<Step> {
    vec![
        Step::Agent {
            agent: AgentStep::new(
                "Analyze code structure for refactoring task ${task}. Identify patterns and issues.",
            )
            .with_model("haiku")
            .with_print()
            .with_max_turns(5)
            .with_tools(vec!["Read".into(), "Grep".into(), "Glob".into()])
            .with_no_session_persistence()
            .with_include_partial_messages(),
        },
        Step::Agent {
            agent: AgentStep::new(
                "Apply refactoring based on the analysis. Make changes incrementally and verify each step.",
            )
            .with_model("sonnet")
            .with_print()
            .with_max_turns(20)
            .with_tools(vec!["Read".into(), "Edit".into(), "Bash".into()])
            .with_no_session_persistence()
            .with_include_partial_messages(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_code_review() {
        let pipeline = get("code-review");
        assert!(pipeline.is_some());
        assert_eq!(pipeline.unwrap().len(), 2);
    }

    #[test]
    fn test_get_merge() {
        let pipeline = get("merge");
        assert!(pipeline.is_some());
        assert_eq!(pipeline.unwrap().len(), 1);
    }

    #[test]
    fn test_get_refactor() {
        let pipeline = get("refactor");
        assert!(pipeline.is_some());
        assert_eq!(pipeline.unwrap().len(), 2);
    }

    #[test]
    fn test_get_unknown() {
        assert!(get("unknown").is_none());
    }
}
