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
├── constants.rs      # 路径常量 (TASKS_DIR, STATUS_FILE, DEFAULT_SESSION_NAME)
├── display.rs        # 显示格式化 (颜色常量, colored_index, running_icon, format_duration)
├── error.rs          # 错误类型 (WtError)
├── models/
│   ├── task.rs       # Task, TaskStatus, TaskInput, Instance
│   ├── status.rs     # StatusStore, TaskState (运行时状态)
│   ├── store.rs      # TaskStore (加载任务 + 状态)
│   ├── config.rs     # WtConfig, HooksConfig (.wt/config.yaml 解析)
│   └── hook_context.rs # HookContext (hook 变量展开)
├── commands/         # 各子命令实现
│   ├── init.rs
│   ├── create.rs
│   ├── validate.rs
│   ├── list.rs
│   ├── next.rs
│   ├── run.rs        # 启动任务，支持 --all 批量启动
│   ├── review.rs     # 标记待审核
│   ├── resume.rs     # 从 Review 恢复到 Running
│   ├── complete.rs   # 完成任务（hooks + merge + cleanup）
│   ├── delete.rs     # 删除任务资源
│   ├── reset.rs      # 重置任务到 Pending
│   ├── new.rs        # scratch 环境创建
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
│   ├── hooks.rs      # HooksEngine (hook 执行)
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

### 配置文件 (.wt/config.yaml)

```yaml
# Terminal multiplexer: tmux (默认) 或 zellij
multiplexer: tmux

# Session 名称
session_name: project-name

# Claude CLI 命令（默认: claude）
claude_command: claude

# wt run 执行的参数
start_args: --verbose --output-format=stream-json -p "@.wt/tasks/${task}.md ..."

# 其他可选配置
worktree_dir: .wt/worktrees
copy_files:
  - .env

# 日志过滤配置 (wt logs)
logs:
  exclude_types: [system, progress]
  exclude_fields: [signature, uuid]

# Hooks - 在任务生命周期各阶段执行脚本
hooks:
  on_create: npm install           # worktree 创建后
  before_review: npm run lint      # review 前检查
  before_complete: npm run build   # 完成前验证
  before_delete: rm -rf node_modules/  # 删除前清理
  before_reset: rm -rf node_modules/   # 重置前清理
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
      "status": "running",
      "instance": {
        "branch": "wt/auth-abc123",
        "worktree_path": ".wt/worktrees/auth",
        "session_name": "wt",
        "window_name": "auth",
        "multiplexer": "tmux"
      }
    },
    "database": { "status": "completed" }
  }
}
```

### TaskStatus 状态流转

```
○ Pending  →  ● Running  →  ? Review  →  ✓ Completed
  (wt run)    (wt review)   (wt complete)
                   ↑            │
                   └────────────┘  (wt resume)
```

- `wt review` 标记任务待审核，关闭 multiplexer 窗口
- `wt resume` 从 Review 恢复到 Running，继续开发
- `wt complete` 执行 hooks + merge + cleanup
- `wt reset` 可从任意状态回到 Pending（会备份代码到 `.wt/backups/`）

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
