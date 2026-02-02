# Handoff 文档 - wt 开发进度

## Session 25 完成的工作 (2026-02-03)

### 1. 术语统一 - Running/Review → Active/Idle

全面统一代码中的术语：

| 类别 | 旧名称 | 新名称 |
|------|--------|--------|
| 错误类型 | `AlreadyRunning` | `AlreadyActive` |
| 方法名 | `mark_review` | `mark_idle` |
| 方法名 | `can_mark_review` | `can_mark_idle` |
| 方法名 | `auto_mark_review_if_needed` | `auto_mark_idle_if_needed` |
| 显示文本 | "Running" | "Active" |
| 显示文本 | "Review" | "Idle" |
| 变量名 | `running` | `active` |
| 变量名 | `review` | `idle` |

涉及文件：list.rs, ui.rs, app.rs, mod.rs, store.rs, error.rs, delete.rs, status/display.rs, status/actions.rs

### 2. Agent Step 参数扩展

根据 Claude CLI 官方文档，扩展了 agent step 支持的参数：

```jsonc
{
  "type": "agent",

  // 基础
  "interactive": false,
  "model": "sonnet",
  "prompt": "...",

  // System Prompt (新增)
  "system_prompt": "...",
  "system_prompt_file": ".wt/prompts/merge.md",
  "append_system_prompt": "...",
  "append_system_prompt_file": ".wt/prompts/code-review.md",

  // Tools
  "tools": ["Read", "Edit"],
  "allowed_tools": ["Bash(git *)"],
  "disallowed_tools": ["Write"],  // 新增

  // Permissions
  "skip_permissions": false,
  "permission_mode": "plan",  // 新增

  // Limits (新增)
  "max_turns": 20,
  "max_budget_usd": 5.0,

  // Session (新增)
  "continue": false,
  "resume": "session-id",

  // I/O
  "output_format": "text",
  "input_format": "stream-json",  // 新增

  // Other (新增)
  "add_dir": ["../lib"],
  "mcp_config": "./mcp.json",
  "verbose": true
}
```

### 3. 示例 Prompts 文件

创建了 `.wt/prompts/` 目录和示例文件：

- `merge.md` - Git merge assistant，指导 agent 执行 rebase + squash merge
- `code-review.md` - Code review guidelines，指导 agent 做代码审核

### 4. 工作流设计确定

```
wt run      → agent 开发 (interactive)
wt review   → lint + build + AI code review (append_system_prompt_file)
wt complete → agent 执行 rebase + merge (append_system_prompt_file)
```

关键点：
- merge 在 `wt complete` 阶段执行
- 使用 agent 处理 rebase 和冲突解决
- 通过 `append_system_prompt_file` 加载指导 prompt

### 5. Demo 项目更新

更新了 `/Users/yansir/code/nextjs-project/try-wt/` 配置：
- 清空旧配置，重新 `wt init`
- 配置使用 bun 作为包管理器
- 配置 review 和 complete hooks 使用 system prompt files

### 测试状态

```
cargo test --lib: 162 passed
cargo test --test cli: 121 passed
cargo test --test integration: 46 passed
Total: 所有测试通过
```

### 提交记录

```
8031f63 feat: extend Agent step with full Claude CLI parameters
24fb83a refactor: unify terminology Running/Review to Active/Idle
```

---

## 项目状态

### 当前架构

```
src/
├── models/
│   ├── config.rs     # JSONC 配置 (.wt/config.jsonc)
│   ├── status.rs     # 状态模型 (Pending/Active/Idle/Completed)
│   ├── store.rs      # 任务存储
│   └── task.rs       # 任务定义
├── services/
│   ├── hooks/        # Hooks 引擎
│   │   ├── mod.rs    # HooksEngine
│   │   ├── context.rs # ExecutionContext
│   │   ├── step.rs   # StepExecutor (支持完整 Claude CLI 参数)
│   │   └── pipeline.rs # PipelineExecutor
│   ├── multiplexer/  # tmux/zellij 抽象
│   └── git.rs        # Git 操作
└── commands/         # CLI 命令
```

### Agent Step 完整参数列表

| 类别 | 参数 | CLI Flag |
|------|------|----------|
| 基础 | `interactive` | `-p` (false) |
| | `model` | `--model` |
| | `prompt` | positional |
| System Prompt | `system_prompt` | `--system-prompt` |
| | `system_prompt_file` | `--system-prompt-file` |
| | `append_system_prompt` | `--append-system-prompt` |
| | `append_system_prompt_file` | `--append-system-prompt-file` |
| Tools | `tools` | `--tools` |
| | `allowed_tools` | `--allowedTools` |
| | `disallowed_tools` | `--disallowedTools` |
| Permissions | `skip_permissions` | `--dangerously-skip-permissions` |
| | `permission_mode` | `--permission-mode` |
| Limits | `max_turns` | `--max-turns` |
| | `max_budget_usd` | `--max-budget-usd` |
| Session | `continue` | `--continue` |
| | `resume` | `--resume` |
| I/O | `output_format` | `--output-format` |
| | `input_format` | `--input-format` |
| Other | `add_dir` | `--add-dir` |
| | `mcp_config` | `--mcp-config` |
| | `verbose` | `--verbose` |

### 规格完成度

| 规格项 | 状态 |
|--------|------|
| JSONC 配置解析 | ✅ |
| 新状态模型 (Pending/Active/Idle/Completed) | ✅ |
| Phase 字段 | ✅ |
| IdleReason 字段 | ✅ |
| active_since 字段 | ✅ |
| script step | ✅ |
| agent step (交互/非交互) | ✅ |
| agent step 完整 CLI 参数 | ✅ |
| internal step | ✅ |
| condition step | ✅ |
| Pipeline 执行器 | ✅ |
| `wt hooks run <hook>` | ✅ |
| `wt pause <task>` | ✅ |
| `wt status --verbose` | ✅ |
| 命令通过新 Hooks API | ✅ |
| 术语统一 (Active/Idle) | ✅ |

---

## 下一步工作

1. **实际测试 complete 工作流** - 在 demo 项目中运行完整的 run → review → complete 流程
2. **internal step 实现完善** - 目前部分 internal 操作是占位符
3. **清理 dead code warnings** - 有一些未使用的辅助方法
4. **Pipeline 测试** - 测试多 agent stream-json 串联

---

## 历史 Session 摘要

| Session | 主要工作 |
|---------|----------|
| 25 | 术语统一 + Agent step 完整 CLI 参数 + prompts 示例 |
| 24 | 完全迁移 + 命令层统一使用新 Hooks API + --verbose |
| 23 | Agent Hooks 系统实现 (Phase 1-4) |
| 22 | Agent Hooks 系统设计（访谈 + Codex 辩论） |
| 21 | cleanup-legacy + Phase 4 docs 完成 |
| 20 | Phase 3 完成、修复 zellij 合并问题 |
| 19 | 补全 atomic-misc CLI 子命令、创建 hooks.rs |
| 18 | Hooks 系统设计、任务规划、Phase 1-2 服务层 |
