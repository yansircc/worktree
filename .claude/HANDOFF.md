# Handoff 文档 - wt 开发进度

## Session 36 完成的工作 (2026-02-03)

### TUI v2 重构 Phase 1 + Phase 2 完成

**新布局**：左右分栏设计
```
┌─ wt ────────────────────┬────────────────────────────────────────────┐
│  1 ● auth      dev  45% │ auth: developing                           │
│  2 ● database  rev  23% │ ─────────────────────────────────────────  │
│  3 ◐ ui        dev  80% │ on_enter workflow                          │
│  4 ○ config    pnd    - │ └─ agent  ●  12m                           │
│                         │                                            │
│                         │ Context   [████████░░░░]  45%              │
│                         │ Duration  12m                              │
│                         │ Git       3 commits  +120/-30              │
│                         │ Tool      Edit                             │
│                         │                                            │
│                         │ ─────────────────────────────────────────  │
│                         │ > Editing src/commands/status.rs...        │
├─────────────────────────┴────────────────────────────────────────────┤
│ j/k select  ⏎ attach  n next  s stop  l log  t tail  q quit          │
└──────────────────────────────────────────────────────────────────────┘
```

**改动文件**：
- `src/tui/ui.rs` - 完全重写为左右分栏布局
- `src/tui/app.rs` - 添加新字段（phase, latest_message, idle_reason, dependencies）
- `src/tui/mod.rs` - run() 接受 show_all 参数
- `src/services/transcript.rs` - 添加 `get_latest_message()`
- `src/models/status.rs` - 添加 `get_phase()`, `get_idle_reason()`，TaskStatus 添加 Copy trait
- `src/models/store.rs` - 添加转发方法
- `src/cli.rs` - 添加 `--all` 参数
- `src/main.rs` / `src/commands/status/mod.rs` - 传递参数

**新功能**：
- 显示所有任务（包括 Pending），不只是 Active/Idle
- `--all` 参数显示 Completed 任务
- 右侧详情面板显示 workflow 进度、context、git 统计、最新消息
- 最新消息作为「心跳」指示器

**Phase 2 - 新增快捷键**：
- `n` - 执行 `wt next`（推进阶段）
- `s` - 执行 `wt stop`（停止进程）
- `l` - 在新 tmux 窗口打开 log

**清理旧代码**：
- 移除 `r`/`u`/`c` 快捷键
- 移除 `can_mark_idle()`/`mark_idle()`/`can_resume()`/`can_complete()`/`mark_completed()` 方法
- 重写 `actions.rs` 使用 `wt stop`/`wt next` 命令

---

## 项目状态

### 测试

```
cargo test --lib: 229 passed ✅
```

### 当前可用命令

```bash
# 任务管理
wt init              # 初始化项目
wt create            # 创建任务
wt validate          # 验证任务
wt list              # 列出任务
wt delete            # 删除任务

# 阶段控制 (Phases v2)
wt next <task>       # 推进到下一阶段 (创建资源 + 启动 agent)
wt prev <task>       # 回退到上一阶段 (执行 on_exit + 清理资源)
wt stop <task>       # 停止任务进程 (支持 --kill-window)
wt reset <task>      # 重置任务 (支持 --to 参数)
wt step done/block/fail  # Agent 标记 step 状态

# 状态和日志
wt status            # 查看状态 (TUI)
wt status --all      # 显示所有任务包括 completed
wt status --json     # JSON 输出
wt tail <task>       # 查看 transcript
wt logs              # 生成日志

# 其他
wt new               # 创建 scratch 环境
wt completions       # Shell 补全
wt internal          # 内部命令
```

---

## 下一步工作

### TUI v2 可选增强

- 显示真实的 workflow/step 进度（当前是占位符）
- 添加 `p` 快捷键（回退阶段）
- 改进 Enter 对 Idle 任务的行为（打开 worktree shell）

### Phase 9: 高级功能
| 子阶段 | 目标 | 状态 |
|--------|------|------|
| 9.1 | 并发执行 - DAG 并行、多任务并行 | 待做 |
| 9.2 | 条件分支 - condition step | 待做 |
| 9.3 | 错误恢复 - on_error、重试、断点续执行 | 待做 |

---

## Phases v2 重构完成状态

| Phase | 状态 | 内容 |
|-------|------|------|
| Phase 1 | ✅ | 核心模型 (step/workflow/phase/project/state) |
| Phase 2 | ✅ | 执行引擎 (executor/observer) |
| Phase 3 | ✅ | 状态管理 (config/status/store v2 桥接) |
| Phase 4a | ✅ | 新增命令 (step/prev) |
| Phase 4b | ✅ | 重写命令 (next/stop/reset --to) |
| Phase 4c | ✅ | 删除旧命令 |
| Phase 5 | ✅ | 清理旧代码 |
| Phase 6 | ✅ | 配置模型 + Observer 集成 |
| Phase 7 | ✅ | prev/stop/step 命令完善 |
| Phase 8.1 | ✅ | TUI v2 布局重构 |
| Phase 8.2 | ✅ | TUI v2 交互完善 |

---

## 历史 Session 摘要

| Session | 主要工作 |
|---------|----------|
| 36 | **TUI v2 完成** - 左右分栏布局、新快捷键(n/s/l)、清理旧代码 |
| 35 | Phase 6.2c + 6.3 + Phase 7 完成 - Observer 集成 + prev/stop/step 命令完善 |
| 34 | Phase 6.2a/b 完成 - next 命令连接执行引擎 + agent 启动 |
| 33 | Phase 4c+5 完成 - 删除旧命令 + 清理旧代码 |
| 32 | Phase 3+4a+4b 完成 - 状态管理 + step/prev/next/stop/reset 命令 |
| 31 | Phase 1+2 完成 - 核心模型 + 执行引擎 |
| 30 | Phases v2 文件清单 - 详细评估每个文件的处置方式 |
