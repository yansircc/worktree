//! Template constants for wt init.

/// Marker for wt entries in .gitignore
pub const GITIGNORE_MARKER: &str = "# wt - Worktree Task Manager";

/// .gitignore entries for wt
pub const GITIGNORE_ENTRIES: &str = r#"# wt - Worktree Task Manager
# https://github.com/anthropics/wt
.wt/*
!.wt/tasks/
!.wt/config.jsonc
!.wt/config.schema.json
!.wt/hooks/
!.wt/templates/
!.wt/verify.md
"#;

/// Agent self-verification checklist template
pub const VERIFY_MD: &str = r#"# 任务完成自检清单

在标记任务状态前，请完成以下检查：

## 1. 功能完整性

- [ ] 所有需求点都已实现
- [ ] 代码能正常编译/运行
- [ ] 关键路径已测试

## 2. 代码质量

- [ ] 没有明显的 bug 或边界问题
- [ ] 错误处理合理
- [ ] 代码风格符合项目规范

## 3. 状态标记

根据检查结果执行对应命令：

| 情况 | 命令 | 说明 |
|------|------|------|
| 全部通过 | `wt step done` | 当前阶段正常完成 |
| 需要人工审核 | `wt step block "原因说明"` | 等待 human 介入 |
| 无法完成 | `wt step fail "失败原因"` | 任务失败 |

---

**重要**: 必须执行上述命令之一来标记任务状态，然后再退出。
"#;

/// Claude settings template with Stop hook for agent verification
pub const VERIFY_SETTINGS_JSON: &str = r#"{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "node $CLAUDE_PROJECT_DIR/.wt/hooks/verify-stop.cjs"
          }
        ]
      }
    ]
  }
}
"#;

/// Stop hook script for agent self-verification
pub const VERIFY_STOP_CJS: &str = r#"#!/usr/bin/env node
/**
 * wt verify stop hook
 *
 * 在 agent 退出前触发，检查是否已执行 wt step 命令标记状态。
 * 如果没有，提示 agent 阅读验证文档并执行状态标记。
 *
 * 防止无限循环：
 * 1. 检测到 wt step 命令 → 放行
 * 2. 已经提示过（lastPromptedLine > 0）→ 放行
 * 3. 否则提示一次并记录位置
 */

const fs = require("fs");
const path = require("path");
const os = require("os");

let inputData = "";
process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk) => {
  inputData += chunk;
});
process.stdin.on("end", () => {
  try {
    processStopHook(JSON.parse(inputData));
  } catch (error) {
    process.exit(0);
  }
});

function processStopHook(input) {
  const sessionId = input.session_id;
  const transcriptPath = input.transcript_path;

  // 没有 transcript 无法追踪，允许退出
  if (!transcriptPath || !fs.existsSync(transcriptPath)) {
    process.exit(0);
  }

  // 状态文件追踪是否已提示
  const stateFile = path.join(os.tmpdir(), `wt-verify-${sessionId}.json`);

  let state = { lastPromptedLine: 0 };
  if (fs.existsSync(stateFile)) {
    try {
      state = JSON.parse(fs.readFileSync(stateFile, "utf8"));
    } catch (e) {}
  }

  // 读取 transcript
  const content = fs.readFileSync(transcriptPath, "utf8");
  const lines = content.trim().split("\n");

  // 检查整个 transcript 是否已执行 wt step 命令
  let hasWtStep = false;
  for (const line of lines) {
    try {
      const entry = JSON.parse(line);
      if (entry.message?.content) {
        const contents = Array.isArray(entry.message.content)
          ? entry.message.content
          : [entry.message.content];

        for (const c of contents) {
          if (c.type === "tool_use" && c.name === "Bash") {
            const command = c.input?.command || "";
            if (/wt\s+step\s+(done|block|fail)/.test(command)) {
              hasWtStep = true;
              break;
            }
          }
        }
      }
    } catch (e) {}
    if (hasWtStep) break;
  }

  // 已执行 wt step，允许退出并清理状态
  if (hasWtStep) {
    try { fs.unlinkSync(stateFile); } catch (e) {}
    process.exit(0);
  }

  // 已经提示过（lastPromptedLine > 0），允许退出
  if (state.lastPromptedLine > 0) {
    // 清理状态文件
    try { fs.unlinkSync(stateFile); } catch (e) {}
    process.exit(0);
  }

  // 首次触发：记录当前位置并提示
  state.lastPromptedLine = lines.length;
  try {
    fs.writeFileSync(stateFile, JSON.stringify(state), "utf8");
  } catch (e) {}

  console.log(
    JSON.stringify({
      decision: "block",
      reason:
        "任务完成前需要自检。请阅读 @.wt/verify.md 并按要求执行验证，然后调用 `wt step done/block/fail` 标记任务状态。",
    })
  );
  process.exit(0);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_gitignore_entries_exposes_verify_templates() {
        assert!(GITIGNORE_ENTRIES.contains("!.wt/hooks/"));
        assert!(GITIGNORE_ENTRIES.contains("!.wt/templates/"));
        assert!(GITIGNORE_ENTRIES.contains("!.wt/verify.md"));
    }
}
