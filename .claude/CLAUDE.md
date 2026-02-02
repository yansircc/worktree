# wt - Worktree Task Manager

多 agent 并行开发任务管理 CLI 工具。

## 项目概述

wt 通过 git worktree 隔离工作区、terminal multiplexer (tmux/zellij) 管理 agent 进程、依赖关系控制任务执行顺序，实现多个 AI agent 并行开发不同功能分支。

## 目录结构

```
src/
├── main.rs           # CLI 入口
├── lib.rs            # 库导出
├── cli.rs            # Clap 命令定义
├── constants.rs      # 路径常量 (TASKS_DIR, DEFAULT_SESSION_NAME)
├── display.rs        # 显示格式化 (颜色常量, colored_index, running_icon, format_duration)
├── error.rs          # 错误类型 (WtError)
├── models/
│   ├── task.rs       # Task, TaskFrontmatter, TaskInput, Instance
│   ├── status.rs     # StatusStore, TaskState, TaskStatus, TaskPhase, IdleReason
│   ├── store.rs      # TaskStore (加载任务 + 状态)
│   └── config.rs     # WtConfig, HooksConfig, Step, HookDef (.wt/config.jsonc 解析)
├── commands/         # 各子命令实现
│   ├── init.rs
│   ├── create.rs
│   ├── validate.rs
│   ├── list.rs
│   ├── next.rs
│   ├── run.rs        # 启动任务，支持 --all 批量启动
│   ├── review.rs     # 标记待审核
│   ├── resume.rs     # 继续开发
│   ├── pause.rs      # 暂停任务 (Active → Idle)
│   ├── complete.rs   # 完成任务（hooks + merge + cleanup）
│   ├── delete.rs     # 删除任务资源
│   ├── reset.rs      # 重置任务到 Pending
│   ├── new.rs        # scratch 环境创建
│   ├── hooks_cmd.rs  # hooks 子命令 (list, run)
│   ├── status/       # 状态命令（已模块化）
│   │   ├── mod.rs    # 入口
│   │   ├── types.rs  # 数据结构
│   │   ├── display.rs # 显示逻辑
│   │   └── actions.rs # Action API
│   ├── tail.rs
│   ├── logs.rs
│   └── completions.rs # shell 补全生成/安装
├── services/
│   ├── command.rs    # 命令执行辅助 (CommandRunner)
│   ├── git.rs        # git worktree 操作
│   ├── hooks/        # Hooks 引擎
│   │   ├── mod.rs    # HooksEngine 主引擎
│   │   ├── context.rs # ExecutionContext (变量展开)
│   │   ├── step.rs   # StepExecutor (script/agent/internal/condition)
│   │   └── pipeline.rs # PipelineExecutor (stream-json 串联)
│   ├── multiplexer/  # terminal multiplexer 抽象层
│   │   ├── mod.rs    # Multiplexer trait + 工厂函数
│   │   ├── tmux.rs   # TmuxBackend 实现
│   │   └── zellij.rs # ZellijBackend 实现
│   ├── workspace.rs  # worktree 初始化 (WorkspaceInitializer)
│   ├── transcript.rs # Claude transcript 解析
│   └── dependency.rs # 依赖检查
└── tui/
    ├── mod.rs        # TUI 入口和事件处理
    ├── app.rs        # TUI 应用状态
    └── ui.rs         # TUI 渲染
```

## 核心概念

### 配置文件 (.wt/config.jsonc)

JSONC 格式，支持注释：

```jsonc
{
  // Terminal multiplexer: tmux (默认) 或 zellij
  "multiplexer": "tmux",

  // Session 名称
  "session_name": "project-name",

  // Claude CLI 命令（默认: claude）
  "claude_command": "claude",

  // Worktree 目录
  "worktree_dir": ".wt/worktrees",

  // Hooks - 每个命令的行为定义
  "hooks": {
    "run": [
      { "type": "script", "run": "npm install" },
      { "type": "agent", "interactive": true, "prompt": "@.wt/tasks/${task}.md" }
    ],
    "review": [
      { "type": "script", "run": "npm run lint && npm run test" }
    ],
    "complete": [
      { "type": "internal", "run": "branch:merge" },
      { "type": "internal", "run": "worktree:destroy" }
    ]
  }
}
```

### Task（任务）

**定义**存储在 `.wt/tasks/*.md`：

```yaml
name: auth          # 任务名（= 文件名，= git 分支名 wt/<name>）
depends:            # 依赖的任务列表
  - database
```

**状态**存储在 `.wt/status.json`：

```json
{
  "tasks": {
    "auth": {
      "status": "active",
      "phase": "developing",
      "idle_reason": null,
      "active_since": "2026-02-03T10:30:00Z",
      "instance": {
        "branch": "wt/auth-abc123",
        "worktree_path": ".wt/worktrees/auth",
        "session_name": "wt",
        "window_name": "auth"
      }
    },
    "database": { "status": "completed" }
  }
}
```

### 状态模型

两个维度描述任务状态：

- **Status** - 资源状态（是否有进程在运行）
- **Phase** - 业务阶段（开发进度）

```
Status:
○ Pending  →  ● Active  ⇄  ◐ Idle  →  ✓ Completed
  (未创建)    (有进程)    (无进程)    (已完成)

Phase:
(none) → developing → reviewing → merging → (done)
```

| Status | Phase | 场景 |
|--------|-------|------|
| Pending | (none) | 任务已定义，未创建资源 |
| Active | developing | agent 正在开发 |
| Idle | developing | agent 暂停，等待用户 |
| Active | reviewing | review pipeline 在运行 |
| Idle | reviewing | review 完成，等待下一步 |
| Completed | (none) | 任务完成 |

### IdleReason

当任务处于 Idle 状态时，说明原因：

| 原因 | 说明 |
|------|------|
| `done` | 当前阶段正常完成 |
| `human_review` | 等待人工审核 |
| `error` | 命令执行出错 |
| `conflict` | 合并冲突待解决 |
| `timeout` | 执行超时 |
| `manual` | 用户手动暂停 |

### Hooks 系统

每个命令的行为由 hooks 定义。Step 类型：

| 类型 | 说明 |
|------|------|
| `script` | 执行 shell 脚本 |
| `agent` | 运行 Claude agent |
| `internal` | 调用 wt 内置操作 |
| `condition` | 条件判断 |

Pipeline 模式支持多 agent stream-json 串联。

### 任务索引

所有命令支持用 1-based 索引代替任务名：
- `wt run 1` 等同于 `wt run <第一个任务>`
- 优先级：任务名 > 索引号（若任务名为 "1"，则匹配任务名）

### 依赖规则

- 任务只能在所有依赖都 `Completed` 后才能 `run`
- `validate` 会检测循环依赖
- `reset` 会在清理前备份代码到 `.wt/backups/`

## 常用命令

```bash
cargo build --release    # 编译
cargo test               # 运行测试
cargo install --path .   # 安装到 ~/.cargo/bin
```

## 任务名验证规则

任务名必须是有效的 git 分支名：
- 不能为空
- 不能含空格、制表符
- 不能含 `~ ^ : ? * [ @ {`
- 不能以 `-` 或 `.` 开头
- 不能以 `.` 或 `.lock` 结尾
- 不能含 `..`

## 相关文件

- @README.md - 用户文档
- @.claude/rules/rust-style.md - Rust 编码规范
- @.claude/rules/testing.md - 测试指南
- @.claude/rules/cli/commands.md - CLI 命令实现规范
- @.claude/specs/agent-hooks.md - Agent Hooks 规格文档
