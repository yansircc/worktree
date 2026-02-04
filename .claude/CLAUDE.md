# wt - Worktree Task Manager

Rust CLI 工具，通过 git worktree + tmux/zellij 实现多 AI agent 并行开发。

## 架构

```
src/
├── main.rs / lib.rs         # 入口和库导出
├── cli.rs                   # Clap 命令定义
├── constants.rs             # 路径常量
├── display.rs               # 终端格式化
├── error.rs                 # WtError 错误类型
│
├── models/                  # 数据模型和业务逻辑
│   ├── config.rs            # WtConfig (JSONC 解析)
│   ├── task.rs              # Task, TaskFrontmatter, Instance
│   ├── task_parser.rs       # YAML frontmatter 解析
│   ├── task_resolver.rs     # 任务名/索引解析
│   ├── status.rs            # TaskStatus, TaskState, StatusStore
│   ├── phase.rs             # Phase, PhaseState, PhaseResources
│   ├── step.rs              # Step, StepState, StepResult, StepVerify
│   ├── workflow.rs          # Workflow, WorkflowState, ExecutionMode
│   ├── state.rs             # TaskRuntimeState (状态派生)
│   ├── agent_step.rs        # AgentStep (Claude CLI 配置)
│   ├── project.rs           # PhasesConfig, ConcurrencyConfig
│   ├── store.rs             # TaskStore (任务加载/缓存)
│   ├── action.rs            # UserAction 枚举
│   ├── schema.rs            # JSON Schema 生成
│   └── validator.rs         # 配置/依赖验证
│
├── commands/                # CLI 命令 (薄层：参数解析 + 调用 models/services)
│   ├── init/                # wt init (config.rs, templates.rs)
│   ├── create.rs            # wt create
│   ├── list.rs              # wt list
│   ├── validate.rs          # wt validate
│   ├── next.rs              # wt next (推进阶段)
│   ├── prev.rs              # wt prev (回退阶段)
│   ├── stop.rs              # wt stop (停止进程)
│   ├── reset.rs             # wt reset (重置任务)
│   ├── delete.rs            # wt delete
│   ├── step.rs              # wt step done/block/fail
│   ├── new.rs               # wt new (临时 worktree)
│   ├── status/              # wt status (display.rs, actions.rs, types.rs)
│   ├── tail.rs              # wt tail
│   ├── logs.rs              # wt logs
│   ├── completions.rs       # shell 补全
│   └── internal/            # wt internal (git/mux/files 原子操作)
│
├── services/                # 外部依赖封装
│   ├── git.rs               # git worktree/branch/metrics
│   ├── claude.rs            # Claude CLI 命令构建
│   ├── command.rs           # 命令执行封装
│   ├── multiplexer/         # tmux/zellij 抽象层
│   ├── executor/            # 执行引擎
│   │   ├── context.rs       # 变量展开、执行上下文
│   │   ├── phase.rs         # Phase 转换
│   │   ├── step.rs          # Step 执行
│   │   ├── workflow.rs      # Workflow 编排
│   │   └── condition/       # 条件表达式解析器
│   ├── observer/            # 观测系统 (log/terminal/sync)
│   ├── dependency.rs        # 依赖关系检查
│   ├── transcript.rs        # Transcript 解析
│   ├── action_resolver.rs   # 可用操作判定
│   ├── task_context.rs      # 任务环境
│   ├── files.rs             # 备份/清理
│   └── workspace.rs         # 工作区管理
│
└── tui/                     # TUI 界面 (ratatui + crossterm)
    ├── app.rs               # 应用状态和刷新
    └── ui.rs                # UI 渲染
```

## 分层原则

| 层 | 职责 | 规则 |
|----|------|------|
| `commands/` | 参数解析、输出格式化 | 不含业务逻辑 |
| `models/` | 数据结构、状态派生、业务逻辑 | 不依赖外部命令 |
| `services/` | git、multiplexer、执行引擎 | 封装外部依赖 |

## 开发命令

```bash
cargo build --release        # 编译
cargo test                   # 全部测试 (单元 + 集成 + CLI)
cargo test --lib             # 仅单元测试
cargo test --test cli        # 仅 CLI E2E 测试
cargo install --path .       # 安装到 ~/.cargo/bin
```

## 相关文档

- @.claude/rules/concepts.md — 核心概念 (状态模型、派生链、生命周期)
- @.claude/rules/api.md — 命令参考 (CLI + TUI + 配置格式)
- @.claude/rules/rust-style.md — Rust 编码规范
- @.claude/rules/testing.md — 测试指南
- @.claude/rules/cli/commands.md — CLI 命令实现规范
