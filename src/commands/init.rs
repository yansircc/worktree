use std::env;
use std::fs;
use std::path::Path;

use crate::constants::{CONFIG_FILE, TASKS_DIR};
use crate::error::{Result, WtError};

const GITIGNORE_MARKER: &str = "# wt - Worktree Task Manager";

const GITIGNORE_ENTRIES: &str = r#"# wt - Worktree Task Manager
# https://github.com/anthropics/wt
.wt/*
!.wt/tasks/
!.wt/config.yaml
"#;

fn get_project_name() -> String {
    env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "wt".to_string())
}

fn generate_config(project_name: &str) -> String {
    format!(
        r#"# wt 配置文件
# 文档: https://github.com/anthropics/wt

# ============================================
# 主要配置
# ============================================

# wt start 执行的参数
# 支持模板变量: ${{task}}, ${{branch}}, ${{worktree}}
#
# 交互模式（默认）- 启动 REPL 带初始 prompt
start_args: '"@.wt/tasks/${{task}}.md 请完成这个任务"'
#
# 非交互模式 - 执行完自动退出，适合 CI/自动化
# start_args: --verbose --output-format=stream-json --input-format=stream-json -p "@.wt/tasks/${{task}}.md 请完成这个任务"

# ============================================
# 可选配置
# ============================================

# Claude CLI 命令
# 默认: claude
# 如果你使用别名或想添加全局 flags，在这里配置
# 示例: ccc, claude --yolo, /path/to/claude
# claude_command: claude

# tmux session 名称
# 默认: 项目目录名
tmux_session: {}

# Worktree 存放目录
# 默认: .wt/worktrees
# 支持相对路径（相对于项目根目录）和绝对路径
# worktree_dir: .wt/worktrees

# 初始化脚本 (在每个新 worktree 中执行)
# 例如安装依赖、设置环境等
# init_script: |
#   npm install

# 需要复制到 worktree 的文件
# 这些文件不会被 git checkout 带过去
# copy_files:
#   - .env
#   - .env.local

# 归档/重置前的清理脚本
# 用于删除大文件（node_modules 等），减少备份体积
# archive_script: |
#   rm -rf node_modules/
#   rm -rf dist/
#   rm -rf .next/
#   rm -rf target/

# ============================================
# 日志配置 (wt logs)
# ============================================

# 过滤规则，用于 wt logs 命令生成调试日志
# 默认不过滤，取消注释以启用
# logs:
#   exclude_types:
#     - system
#     - progress
#     - file-history-snapshot
#   exclude_fields:
#     - signature
#     - cwd
#     - gitBranch
#     - permissionMode
#     - sessionId
#     - id
#     - model
#     - tool_use_id
#     - timestamp
#     - version
#     - usage
#     - parentUuid
#     - uuid
#     - isSidechain
#     - userType
#     - slug
#     - thinkingMetadata
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

    // Create .wt/config.yaml
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
    println!("  2. Create tasks: wt create --json '{{\"name\": \"...\", \"description\": \"...\"}}'");
    println!("  3. Start working: wt start <task>");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_config_contains_project_name() {
        let config = generate_config("my-project");
        assert!(config.contains("tmux_session: my-project"));
    }

    #[test]
    fn test_generate_config_has_required_fields() {
        let config = generate_config("test");
        assert!(config.contains("start_args:"));
        assert!(config.contains("tmux_session:"));
        assert!(config.contains("worktree_dir:"));
        assert!(config.contains("copy_files:"));
        assert!(config.contains(".env"));
    }

    #[test]
    fn test_generate_config_has_claude_command_comment() {
        let config = generate_config("test");
        assert!(config.contains("claude_command:"));
    }

    #[test]
    fn test_generate_config_has_template_variables() {
        let config = generate_config("test");
        assert!(config.contains("${task}"));
    }

    #[test]
    fn test_generate_config_has_archive_script() {
        let config = generate_config("test");
        assert!(config.contains("archive_script:"));
        assert!(config.contains("node_modules"));
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
        assert!(GITIGNORE_ENTRIES.contains("!.wt/config.yaml"));
    }
}
