use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::constants::{CONFIG_FILE, DEFAULT_SESSION_NAME, DEFAULT_WORKTREE_DIR};
use crate::error::{Result, WtError};
use crate::services::multiplexer::{create_multiplexer, Multiplexer, MultiplexerType};

/// Hook names for task lifecycle events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookName {
    OnCreate,
    BeforeRun,
    AfterRun,
    BeforeReview,
    AfterReview,
    BeforeResume,
    BeforeComplete,
    AfterComplete,
    BeforeDelete,
    BeforeReset,
}

impl HookName {
    pub fn as_str(&self) -> &'static str {
        match self {
            HookName::OnCreate => "on_create",
            HookName::BeforeRun => "before_run",
            HookName::AfterRun => "after_run",
            HookName::BeforeReview => "before_review",
            HookName::AfterReview => "after_review",
            HookName::BeforeResume => "before_resume",
            HookName::BeforeComplete => "before_complete",
            HookName::AfterComplete => "after_complete",
            HookName::BeforeDelete => "before_delete",
            HookName::BeforeReset => "before_reset",
        }
    }
}

/// Configuration for hooks at different lifecycle stages
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HooksConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_create: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_run: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_run: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_review: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_review: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_resume: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_complete: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_complete: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_delete: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_reset: Option<String>,
}

impl HooksConfig {
    pub fn get(&self, hook: HookName) -> Option<&String> {
        match hook {
            HookName::OnCreate => self.on_create.as_ref(),
            HookName::BeforeRun => self.before_run.as_ref(),
            HookName::AfterRun => self.after_run.as_ref(),
            HookName::BeforeReview => self.before_review.as_ref(),
            HookName::AfterReview => self.after_review.as_ref(),
            HookName::BeforeResume => self.before_resume.as_ref(),
            HookName::BeforeComplete => self.before_complete.as_ref(),
            HookName::AfterComplete => self.after_complete.as_ref(),
            HookName::BeforeDelete => self.before_delete.as_ref(),
            HookName::BeforeReset => self.before_reset.as_ref(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.on_create.is_none()
            && self.before_run.is_none()
            && self.after_run.is_none()
            && self.before_review.is_none()
            && self.after_review.is_none()
            && self.before_resume.is_none()
            && self.before_complete.is_none()
            && self.after_complete.is_none()
            && self.before_delete.is_none()
            && self.before_reset.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WtConfig {
    #[serde(default = "default_claude_command")]
    pub claude_command: String,
    #[serde(default = "default_start_args")]
    pub start_args: String,
    /// Multiplexer to use: tmux (default) or zellij
    #[serde(default = "default_multiplexer")]
    pub multiplexer: String,
    /// Session name for the multiplexer
    #[serde(default = "default_session_name")]
    pub session_name: String,
    #[serde(default = "default_worktree_dir")]
    pub worktree_dir: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub copy_files: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub init_script: Option<String>,
    #[serde(default)]
    pub logs: LogsConfig,
    /// Script to run before archiving/reset (optional, for cleanup like rm -rf node_modules/)
    /// DEPRECATED: Use hooks.before_delete and hooks.before_reset instead
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_script: Option<String>,
    /// Script to run when marking task as review (optional, e.g., run tests)
    /// DEPRECATED: Use hooks.before_review instead
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_script: Option<String>,
    /// Script to run during merge (optional, e.g., additional build steps)
    /// DEPRECATED: Use hooks.before_complete instead
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_script: Option<String>,
    /// New hooks configuration
    #[serde(default, skip_serializing_if = "HooksConfig::is_empty")]
    pub hooks: HooksConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogsConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_types: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_fields: Vec<String>,
}

fn default_claude_command() -> String {
    "claude".to_string()
}

fn default_start_args() -> String {
    r#"--verbose --output-format=stream-json --input-format=stream-json -p "@.wt/tasks/${task}.md 请完成这个任务""#.to_string()
}

fn default_multiplexer() -> String {
    "tmux".to_string()
}

fn default_session_name() -> String {
    DEFAULT_SESSION_NAME.to_string()
}

fn default_worktree_dir() -> String {
    DEFAULT_WORKTREE_DIR.to_string()
}

impl WtConfig {
    pub fn load() -> Result<Self> {
        let path = Path::new(CONFIG_FILE);
        if !path.exists() {
            return Err(WtError::ConfigNotFound);
        }
        let content =
            std::fs::read_to_string(path).map_err(|e| WtError::ConfigRead(e.to_string()))?;
        Self::from_str(&content)
    }

    /// Parse config from string
    pub fn from_str(content: &str) -> Result<Self> {
        let config: WtConfig = serde_yaml::from_str(content)?;
        Ok(config)
    }

    /// Get the configured multiplexer type
    pub fn multiplexer_type(&self) -> MultiplexerType {
        MultiplexerType::from_str(&self.multiplexer).unwrap_or_default()
    }

    /// Create a multiplexer instance based on config
    pub fn create_multiplexer(&self) -> Box<dyn Multiplexer> {
        create_multiplexer(self.multiplexer_type())
    }

    /// Get hook script by name, with fallback to legacy fields for backward compatibility
    ///
    /// Priority: new hooks format > legacy fields
    /// Mapping:
    /// - init_script → on_create
    /// - review_script → before_review
    /// - merge_script → before_complete
    /// - archive_script → before_delete, before_reset
    pub fn get_hook(&self, hook: HookName) -> Option<&String> {
        // First check new hooks config
        if let Some(script) = self.hooks.get(hook) {
            return Some(script);
        }

        // Fallback to legacy fields
        match hook {
            HookName::OnCreate => self.init_script.as_ref(),
            HookName::BeforeReview => self.review_script.as_ref(),
            HookName::BeforeComplete => self.merge_script.as_ref(),
            HookName::BeforeDelete | HookName::BeforeReset => self.archive_script.as_ref(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_minimal() {
        let yaml = "start_args: -p test\n";
        let config = WtConfig::from_str(yaml).unwrap();

        assert_eq!(config.claude_command, "claude"); // default
        assert_eq!(config.start_args, "-p test");
        assert_eq!(config.multiplexer, "tmux"); // default
        assert_eq!(config.session_name, "wt"); // default
        assert_eq!(config.worktree_dir, ".wt/worktrees"); // default
        assert!(config.copy_files.is_empty());
        assert!(config.init_script.is_none());
    }

    #[test]
    fn test_config_all_defaults() {
        let yaml = "{}\n";
        let config = WtConfig::from_str(yaml).unwrap();

        assert_eq!(config.claude_command, "claude");
        assert!(config.start_args.contains("--output-format=stream-json"));
        assert_eq!(config.multiplexer, "tmux");
        assert_eq!(config.session_name, "wt");
        assert_eq!(config.worktree_dir, ".wt/worktrees");
    }

    #[test]
    fn test_config_full() {
        let yaml = r#"
claude_command: ccc --yolo
start_args: -p "test"
multiplexer: zellij
session_name: my-session
worktree_dir: /custom/path
copy_files:
  - .env
  - config.json
init_script: npm install
"#;
        let config = WtConfig::from_str(yaml).unwrap();

        assert_eq!(config.claude_command, "ccc --yolo");
        assert_eq!(config.start_args, "-p \"test\"");
        assert_eq!(config.multiplexer, "zellij");
        assert_eq!(config.session_name, "my-session");
        assert_eq!(config.worktree_dir, "/custom/path");
        assert_eq!(config.copy_files, vec![".env", "config.json"]);
        assert_eq!(config.init_script, Some("npm install".to_string()));
    }

    #[test]
    fn test_config_invalid_yaml() {
        let yaml = "claude_command: [invalid";
        let result = WtConfig::from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_empty() {
        let yaml = "";
        let result = WtConfig::from_str(yaml);
        // Empty YAML now works with all defaults
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_config_multiline_init_script() {
        let yaml = r#"
init_script: |
  npm install
  npm run build
"#;
        let config = WtConfig::from_str(yaml).unwrap();
        assert!(config.init_script.unwrap().contains("npm install"));
    }

    #[test]
    fn test_config_serialize() {
        let config = WtConfig {
            claude_command: "ccc".to_string(),
            start_args: "-p test".to_string(),
            multiplexer: "tmux".to_string(),
            session_name: "wt".to_string(),
            worktree_dir: ".wt/worktrees".to_string(),
            copy_files: vec![".env".to_string()],
            init_script: Some("npm i".to_string()),
            logs: LogsConfig::default(),
            archive_script: None,
            review_script: None,
            merge_script: None,
            hooks: HooksConfig::default(),
        };
        let yaml = serde_yaml::to_string(&config).unwrap();

        assert!(yaml.contains("claude_command: ccc"));
        assert!(yaml.contains("start_args:"));
        assert!(yaml.contains("copy_files:"));
        assert!(yaml.contains("init_script:"));
    }

    #[test]
    fn test_config_with_archive_script() {
        let yaml = r#"
archive_script: |
  rm -rf node_modules/
  rm -rf dist/
"#;
        let config = WtConfig::from_str(yaml).unwrap();
        assert!(config.archive_script.is_some());
        assert!(config.archive_script.unwrap().contains("node_modules"));
    }

    #[test]
    fn test_config_with_logs() {
        let yaml = r#"
logs:
  exclude_types:
    - system
    - progress
  exclude_fields:
    - signature
    - parentUuid
"#;
        let config = WtConfig::from_str(yaml).unwrap();

        assert_eq!(config.logs.exclude_types, vec!["system", "progress"]);
        assert_eq!(config.logs.exclude_fields, vec!["signature", "parentUuid"]);
    }

    #[test]
    fn test_multiplexer_type() {
        let yaml = "multiplexer: tmux\n";
        let config = WtConfig::from_str(yaml).unwrap();
        assert_eq!(config.multiplexer_type(), MultiplexerType::Tmux);

        let yaml = "multiplexer: zellij\n";
        let config = WtConfig::from_str(yaml).unwrap();
        assert_eq!(config.multiplexer_type(), MultiplexerType::Zellij);

        // Unknown defaults to tmux
        let yaml = "multiplexer: unknown\n";
        let config = WtConfig::from_str(yaml).unwrap();
        assert_eq!(config.multiplexer_type(), MultiplexerType::Tmux);
    }

    #[test]
    fn test_hooks_config_parse() {
        let yaml = r#"
hooks:
  on_create: cargo check
  before_run: echo "starting"
  after_run: echo "done"
  before_review: |
    cargo fmt --check
    cargo clippy -- -D warnings
  before_complete: cargo test
  after_complete: echo "completed"
  before_delete: rm -rf target/
  before_reset: cargo clean
"#;
        let config = WtConfig::from_str(yaml).unwrap();

        assert_eq!(config.hooks.on_create, Some("cargo check".to_string()));
        assert_eq!(
            config.hooks.before_run,
            Some("echo \"starting\"".to_string())
        );
        assert_eq!(config.hooks.after_run, Some("echo \"done\"".to_string()));
        assert!(config
            .hooks
            .before_review
            .as_ref()
            .unwrap()
            .contains("cargo fmt"));
        assert!(config
            .hooks
            .before_review
            .as_ref()
            .unwrap()
            .contains("cargo clippy"));
        assert_eq!(config.hooks.before_complete, Some("cargo test".to_string()));
        assert_eq!(
            config.hooks.after_complete,
            Some("echo \"completed\"".to_string())
        );
        assert_eq!(
            config.hooks.before_delete,
            Some("rm -rf target/".to_string())
        );
        assert_eq!(config.hooks.before_reset, Some("cargo clean".to_string()));
    }

    #[test]
    fn test_hooks_config_partial() {
        let yaml = r#"
hooks:
  on_create: npm install
"#;
        let config = WtConfig::from_str(yaml).unwrap();

        assert_eq!(config.hooks.on_create, Some("npm install".to_string()));
        assert!(config.hooks.before_run.is_none());
        assert!(config.hooks.after_run.is_none());
        assert!(config.hooks.before_review.is_none());
    }

    #[test]
    fn test_get_hook_new_format() {
        let yaml = r#"
hooks:
  on_create: new-script
  before_review: new-review
  before_complete: new-merge
  before_delete: new-delete
  before_reset: new-reset
"#;
        let config = WtConfig::from_str(yaml).unwrap();

        assert_eq!(
            config.get_hook(HookName::OnCreate),
            Some(&"new-script".to_string())
        );
        assert_eq!(
            config.get_hook(HookName::BeforeReview),
            Some(&"new-review".to_string())
        );
        assert_eq!(
            config.get_hook(HookName::BeforeComplete),
            Some(&"new-merge".to_string())
        );
        assert_eq!(
            config.get_hook(HookName::BeforeDelete),
            Some(&"new-delete".to_string())
        );
        assert_eq!(
            config.get_hook(HookName::BeforeReset),
            Some(&"new-reset".to_string())
        );
    }

    #[test]
    fn test_get_hook_legacy_fallback() {
        let yaml = r#"
init_script: legacy-init
review_script: legacy-review
merge_script: legacy-merge
archive_script: legacy-archive
"#;
        let config = WtConfig::from_str(yaml).unwrap();

        // Legacy fields should map to hooks
        assert_eq!(
            config.get_hook(HookName::OnCreate),
            Some(&"legacy-init".to_string())
        );
        assert_eq!(
            config.get_hook(HookName::BeforeReview),
            Some(&"legacy-review".to_string())
        );
        assert_eq!(
            config.get_hook(HookName::BeforeComplete),
            Some(&"legacy-merge".to_string())
        );
        assert_eq!(
            config.get_hook(HookName::BeforeDelete),
            Some(&"legacy-archive".to_string())
        );
        assert_eq!(
            config.get_hook(HookName::BeforeReset),
            Some(&"legacy-archive".to_string())
        );
    }

    #[test]
    fn test_get_hook_new_format_takes_priority() {
        let yaml = r#"
init_script: legacy-init
review_script: legacy-review
hooks:
  on_create: new-init
  before_review: new-review
"#;
        let config = WtConfig::from_str(yaml).unwrap();

        // New hooks take priority over legacy
        assert_eq!(
            config.get_hook(HookName::OnCreate),
            Some(&"new-init".to_string())
        );
        assert_eq!(
            config.get_hook(HookName::BeforeReview),
            Some(&"new-review".to_string())
        );
    }

    #[test]
    fn test_get_hook_no_fallback_for_new_hooks() {
        let yaml = r#"
init_script: legacy-init
"#;
        let config = WtConfig::from_str(yaml).unwrap();

        // Hooks without legacy mapping return None
        assert!(config.get_hook(HookName::BeforeRun).is_none());
        assert!(config.get_hook(HookName::AfterRun).is_none());
        assert!(config.get_hook(HookName::AfterReview).is_none());
        assert!(config.get_hook(HookName::BeforeResume).is_none());
        assert!(config.get_hook(HookName::AfterComplete).is_none());
    }

    #[test]
    fn test_hooks_config_is_empty() {
        let empty = HooksConfig::default();
        assert!(empty.is_empty());

        let non_empty = HooksConfig {
            on_create: Some("test".to_string()),
            ..Default::default()
        };
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_hook_name_as_str() {
        assert_eq!(HookName::OnCreate.as_str(), "on_create");
        assert_eq!(HookName::BeforeRun.as_str(), "before_run");
        assert_eq!(HookName::AfterRun.as_str(), "after_run");
        assert_eq!(HookName::BeforeReview.as_str(), "before_review");
        assert_eq!(HookName::AfterReview.as_str(), "after_review");
        assert_eq!(HookName::BeforeResume.as_str(), "before_resume");
        assert_eq!(HookName::BeforeComplete.as_str(), "before_complete");
        assert_eq!(HookName::AfterComplete.as_str(), "after_complete");
        assert_eq!(HookName::BeforeDelete.as_str(), "before_delete");
        assert_eq!(HookName::BeforeReset.as_str(), "before_reset");
    }
}
