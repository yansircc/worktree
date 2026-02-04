//! Configuration generation for wt init.

/// Generate default config.jsonc content
pub fn generate_config(project_name: &str) -> String {
    format!(
        r#"{{
  "$schema": "./config.schema.json",

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
  // Phases - 任务生命周期定义
  // ============================================
  // 任务按阶段推进: pending → developing → reviewing → completed
  // 每个阶段可以定义 on_enter (进入时执行) 和 on_exit (退出时执行) 工作流
  // 资源创建/销毁由 steps 中的命令管理
  //
  // 支持模板变量: ${{task}}, ${{branch}}, ${{worktree}}, ${{session}}, ${{window}}, ${{repo_root}}

  "phases": {{
    // 阶段序列（默认）
    "sequence": ["pending", "developing", "reviewing", "completed"],

    // 阶段定义（所有 sequence 中的阶段都必须有定义）
    "definitions": {{
      // pending 阶段 - 初始状态
      // on_exit: 创建 branch 和 worktree
      "pending": {{
        "id": "pending",
        "on_exit": {{
          "steps": [
            {{ "id": "create-branch", "run": "git branch ${{branch}}", "name": "Create branch" }},
            {{ "id": "create-worktree", "run": "git -C ${{repo_root}} worktree add ${{worktree}} ${{branch}}", "name": "Create worktree" }}
          ]
        }}
      }},

      // developing 阶段 - 开发
      // on_enter: 创建 window, 启动 agent
      "developing": {{
        "id": "developing",
        "on_enter": {{
          "steps": [
            {{ "id": "create-window", "run": "tmux new-window -t ${{session}} -n ${{window}} -c ${{worktree}} || true", "name": "Create window" }},
            {{
              "agent": {{
                "prompt": "@.wt/tasks/${{task}}.md 请完成这个任务",
                "model": "sonnet",
                // 启用 agent 自验证（推荐）
                // agent 退出前会自动检查并调用 wt step done/block/fail
                "settings": ".wt/templates/verify-settings.json"
              }},
              "verify": {{ "run": "true" }}
            }}
          ]
        }}
      }},

      // reviewing 阶段 - 审查
      "reviewing": {{
        "id": "reviewing",
        "on_enter": {{
          "steps": [
            {{
              "agent": {{
                "prompt": "审查代码质量和安全性",
                "model": "sonnet"
              }}
            }}
          ]
        }}
      }},

      // completed 阶段 - 终态
      // on_enter: 清理 window, worktree, branch
      "completed": {{
        "id": "completed",
        "terminal": true,
        "on_enter": {{
          "steps": [
            {{ "id": "close-window", "run": "tmux kill-window -t ${{session}}:${{window}} 2>/dev/null || true", "name": "Close window" }},
            {{ "id": "remove-worktree", "run": "git -C ${{repo_root}} worktree remove --force ${{worktree}} 2>/dev/null || true", "name": "Remove worktree" }},
            {{ "id": "delete-branch", "run": "git -C ${{repo_root}} branch -D ${{branch}} 2>/dev/null || true", "name": "Delete branch" }}
          ]
        }}
      }}
    }}
  }}
}}
"#,
        project_name
    )
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
        assert!(config.contains("\"phases\":"));
    }

    #[test]
    fn test_generate_config_has_template_variables() {
        let config = generate_config("test");
        assert!(config.contains("${task}"));
        assert!(config.contains("${branch}"));
        assert!(config.contains("${worktree}"));
        assert!(config.contains("${repo_root}"));
    }

    #[test]
    fn test_generate_config_has_phases() {
        let config = generate_config("test");
        assert!(config.contains("\"sequence\":"));
        assert!(config.contains("\"definitions\":"));
        assert!(config.contains("\"developing\":"));
    }

    #[test]
    fn test_generate_config_has_verify_settings() {
        let config = generate_config("test");
        assert!(config.contains("verify-settings.json"));
    }

    #[test]
    fn test_generate_config_has_resource_steps() {
        let config = generate_config("test");
        // Check for resource creation steps
        assert!(config.contains("create-branch"));
        assert!(config.contains("create-worktree"));
        assert!(config.contains("create-window"));
        // Check for resource cleanup steps
        assert!(config.contains("close-window"));
        assert!(config.contains("remove-worktree"));
        assert!(config.contains("delete-branch"));
    }
}
