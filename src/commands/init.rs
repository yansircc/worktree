use std::env;
use std::fs;
use std::path::Path;

use crate::constants::TASKS_DIR;
use crate::error::{Result, WtError};
use crate::models::CONFIG_FILE;

const GITIGNORE_MARKER: &str = "# wt - Worktree Task Manager";

const GITIGNORE_ENTRIES: &str = r#"# wt - Worktree Task Manager
# https://github.com/anthropics/wt
.wt/*
!.wt/tasks/
!.wt/config.jsonc
"#;

fn get_project_name() -> String {
    env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "wt".to_string())
}

fn generate_config(project_name: &str) -> String {
    format!(
        r#"{{
  // ============================================
  // 基础配置
  // ============================================

  // Terminal multiplexer: tmux (默认) 或 zellij
  "multiplexer": "tmux",

  // Session 名称
  "session_name": "{}",

  // Claude CLI 命令（默认: claude）
  // "claude_command": "claude",

  // Worktree 目录
  // "worktree_dir": ".wt/worktrees",

  // ============================================
  // Hooks - 每个命令的行为定义
  // ============================================
  // 支持的 step 类型:
  // - script: 执行 shell 脚本
  // - agent: 运行 Claude agent
  // - internal: 调用 wt 内置操作
  // - condition: 条件判断
  //
  // 支持模板变量: ${{task}}, ${{branch}}, ${{worktree}}, ${{session}}, ${{window}}

  "hooks": {{
    // wt next: 启动开发
    "run": [
      // {{ "type": "script", "run": "npm install" }},
      {{
        "type": "agent",
        "interactive": true,
        "model": "sonnet",
        "prompt": "@.wt/tasks/${{task}}.md 请完成这个任务"
      }}
    ]

    // wt review: 进入审核阶段
    // "review": [
    //   {{ "type": "script", "run": "npm run lint && npm run test" }}
    // ],

    // wt complete: 完成任务
    // "complete": [
    //   {{ "type": "script", "run": "npm run build" }},
    //   {{ "type": "internal", "run": "branch:merge" }},
    //   {{ "type": "internal", "run": "worktree:destroy" }}
    // ],

    // wt delete/reset: 删除或重置任务
    // "delete": [
    //   {{ "type": "script", "run": "rm -rf node_modules/" }}
    // ]
  }}
}}
"#,
        project_name
    )
}

fn update_gitignore() -> Result<bool> {
    let gitignore_path = Path::new(".gitignore");

    if gitignore_path.exists() {
        let content = fs::read_to_string(gitignore_path).map_err(|e| WtError::Io {
            operation: "read".to_string(),
            path: ".gitignore".to_string(),
            message: e.to_string(),
        })?;

        // Check if already has wt entries
        if content.contains(GITIGNORE_MARKER) {
            return Ok(false);
        }

        // Append to existing .gitignore
        let new_content = if content.ends_with('\n') {
            format!("{}\n{}", content, GITIGNORE_ENTRIES)
        } else {
            format!("{}\n\n{}", content, GITIGNORE_ENTRIES)
        };

        fs::write(gitignore_path, new_content).map_err(|e| WtError::Io {
            operation: "write".to_string(),
            path: ".gitignore".to_string(),
            message: e.to_string(),
        })?;
    } else {
        // Create new .gitignore
        fs::write(gitignore_path, GITIGNORE_ENTRIES).map_err(|e| WtError::Io {
            operation: "create".to_string(),
            path: ".gitignore".to_string(),
            message: e.to_string(),
        })?;
    }

    Ok(true)
}

pub fn execute() -> Result<()> {
    let wt_dir = Path::new(".wt");
    let config_path = Path::new(CONFIG_FILE);
    let tasks_dir = Path::new(TASKS_DIR);

    // Check if already initialized
    if config_path.exists() {
        return Err(WtError::Io {
            operation: "init".to_string(),
            path: CONFIG_FILE.to_string(),
            message: "already exists. Remove it first if you want to reinitialize.".to_string(),
        });
    }

    let project_name = get_project_name();

    // Create .wt/ directory
    if !wt_dir.exists() {
        fs::create_dir(wt_dir).map_err(|e| WtError::Io {
            operation: "create".to_string(),
            path: ".wt".to_string(),
            message: e.to_string(),
        })?;
    }

    // Create .wt/config.jsonc
    let config_content = generate_config(&project_name);
    fs::write(config_path, &config_content).map_err(|e| WtError::Io {
        operation: "create".to_string(),
        path: CONFIG_FILE.to_string(),
        message: e.to_string(),
    })?;
    println!("Created {}", CONFIG_FILE);

    // Create .wt/tasks/ directory
    if !tasks_dir.exists() {
        fs::create_dir_all(tasks_dir).map_err(|e| WtError::Io {
            operation: "create".to_string(),
            path: TASKS_DIR.to_string(),
            message: e.to_string(),
        })?;
        println!("Created {}/", TASKS_DIR);
    }

    // Update .gitignore
    if update_gitignore()? {
        println!("Updated .gitignore");
    } else {
        println!(".gitignore already has wt entries");
    }

    // Install shell completions if not already installed
    if !super::completions::is_installed() {
        println!();
        println!("Installing shell completions...");
        match super::completions::install() {
            Ok(()) => {}
            Err(e) => {
                println!("  Warning: Failed to install completions: {}", e);
                println!("  You can install manually with: wt completions install");
            }
        }
    }

    // Summary
    println!();
    println!("Initialized wt for project '{}'", project_name);
    println!();
    println!("Next steps:");
    println!("  1. Edit {} to customize settings", CONFIG_FILE);
    println!(
        "  2. Create tasks: wt create --json '{{\"name\": \"...\", \"description\": \"...\"}}'"
    );
    println!("  3. Start working: wt next <task>");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_config_contains_project_name() {
        let config = generate_config("my-project");
        assert!(config.contains("\"session_name\": \"my-project\""));
    }

    #[test]
    fn test_generate_config_has_required_fields() {
        let config = generate_config("test");
        assert!(config.contains("\"multiplexer\":"));
        assert!(config.contains("\"session_name\":"));
        assert!(config.contains("\"hooks\":"));
    }

    #[test]
    fn test_generate_config_has_template_variables() {
        let config = generate_config("test");
        assert!(config.contains("${task}"));
    }

    #[test]
    fn test_generate_config_has_hooks() {
        let config = generate_config("test");
        assert!(config.contains("\"run\":"));
        assert!(config.contains("\"type\": \"agent\""));
    }

    #[test]
    fn test_gitignore_entries_has_marker() {
        assert!(GITIGNORE_ENTRIES.contains(GITIGNORE_MARKER));
    }

    #[test]
    fn test_gitignore_entries_has_wt_dir() {
        assert!(GITIGNORE_ENTRIES.contains(".wt/*"));
    }

    #[test]
    fn test_gitignore_entries_exposes_tasks_and_config() {
        assert!(GITIGNORE_ENTRIES.contains("!.wt/tasks/"));
        assert!(GITIGNORE_ENTRIES.contains("!.wt/config.jsonc"));
    }
}
