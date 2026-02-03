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
├── constants.rs      # 路径常量
├── display.rs        # 显示格式化
├── error.rs          # 错误类型
├── models/
│   ├── task.rs       # Task, TaskFrontmatter, Instance
│   ├── status.rs     # StatusStore, TaskState, TaskStatus, TaskPhase
│   ├── store.rs      # TaskStore
│   ├── config.rs     # WtConfig
│   ├── phase.rs      # Phase, PhaseState
│   ├── step.rs       # Step, StepState, StepResult
│   ├── workflow.rs   # Workflow, WorkflowState
│   └── state.rs      # TaskRuntimeState, 状态派生
├── commands/
│   ├── init.rs       # wt init
│   ├── create.rs     # wt create
│   ├── list.rs       # wt list
│   ├── next.rs       # wt next - 推进阶段
│   ├── prev.rs       # wt prev - 回退阶段
│   ├── stop.rs       # wt stop - 停止进程
│   ├── step.rs       # wt step done/block/fail
│   ├── reset.rs      # wt reset
│   ├── delete.rs     # wt delete
│   ├── status/       # wt status (TUI)
│   └── ...
├── services/
│   ├── git.rs        # git worktree 操作
│   ├── claude.rs     # ClaudeCommandBuilder
│   ├── multiplexer/  # tmux/zellij 抽象层
│   ├── executor/     # 执行引擎
│   └── observer/     # 观测系统
└── tui/              # TUI 界面
```

## 常用命令

```bash
cargo build --release    # 编译
cargo test               # 运行测试
cargo install --path .   # 安装到 ~/.cargo/bin
```

## 相关文件

- @.claude/rules/concepts.md - 核心概念（状态模型、派生链）
- @.claude/rules/api.md - 命令参考（CLI、TUI、配置）
- @.claude/rules/rust-style.md - Rust 编码规范
- @.claude/rules/testing.md - 测试指南
- @.claude/rules/cli/commands.md - CLI 命令实现规范
