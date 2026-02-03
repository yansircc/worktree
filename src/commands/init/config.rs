//! Configuration generation for wt init.

/// Generate default config.jsonc content
pub fn generate_config(project_name: &str) -> String {
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
  // Phases - 任务生命周期定义
  // ============================================
  // 任务按阶段推进: pending → developing → reviewing → completed
  // 每个阶段可以定义 on_enter (进入时执行) 和 on_exit (退出时执行) 工作流
  //
  // 支持模板变量: ${{task}}, ${{branch}}, ${{worktree}}, ${{session}}, ${{window}}

  "phases": {{
    // 阶段序列（默认）
    "sequence": ["pending", "developing", "reviewing", "completed"],

    // 阶段定义
    "definitions": {{
      // developing 阶段 - 需要资源（worktree, branch, window）
      "developing": {{
        "id": "developing",
        "resources": "full",
        "on_enter": {{
          "steps": [
            {{
              "agent": {{
                "prompt": "@.wt/tasks/${{task}}.md 请完成这个任务",
                "model": "sonnet",
                // 启用 agent 自验证（推荐）
                // agent 退出前会自动检查并调用 wt step done/block/fail
                "settings": ".wt/templates/verify-settings.json"
              }},
              "verify": {{ "type": "self" }}
            }}
          ]
        }}
      }},

      // reviewing 阶段 - 需要资源
      "reviewing": {{
        "id": "reviewing",
        "resources": "full",
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

      // completed 阶段 - 不需要资源
      "completed": {{
        "id": "completed",
        "resources": "none"
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
}
