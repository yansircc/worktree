# Hooks 系统重构 - 背景文档

## 重构目标

将 wt 从"硬编码行为"升级为"可配置的 hooks 系统"，让用户可以通过变量和原子操作自由 DIY 任务生命周期的每个阶段。

## 核心变化

### 1. 配置格式变化

**旧格式：**
```yaml
init_script: cargo check
review_script: cargo test
merge_script: cargo build
archive_script: rm -rf target/
```

**新格式：**
```yaml
hooks:
  on_create: |
    cargo check
  before_review: |
    cargo fmt --check
    cargo clippy -- -D warnings
  before_complete: |
    cargo test
  after_complete: |
    wt::notify "Task ${task} completed!"
  before_delete: |
    wt::files::backup ${task}
  before_reset: |
    rm -rf target/
```

### 2. 命令变化

| 旧命令 | 新命令 | 说明 |
|--------|--------|------|
| `wt start` | `wt run` | 改名，更直观 |
| `wt merge` | `wt complete` | 简化，不再调用 Claude |
| `wt archive` | `wt delete` | 统一删除，支持 completed + scratch |

### 3. 原子操作

提供以下类别的原子操作供用户在 hooks 中调用：

```
wt::git::*      - Git 操作 (create_branch, delete_branch, rebase, squash_merge, ...)
wt::mux::*      - Multiplexer 操作 (create_window, close_window, focus_window, ...)
wt::files::*    - 文件操作 (copy, backup, clean)
wt::claude::*   - Claude 操作 (start, run, is_running)
wt::status::*   - 状态操作 (set, get)
wt::task::*     - 任务操作 (exists, deps_ready, list_blocked)
wt::notify      - 系统通知
wt::log         - 日志记录
wt::confirm     - 交互确认
wt::abort       - 中止操作
wt::config::get - 读取配置
```

### 4. 变量

hooks 脚本中可用的变量：
```
${task}         - 任务名
${branch}       - 分支名
${worktree}     - worktree 路径
${repo_root}    - 主仓库路径
${session}      - multiplexer session 名
${window}       - multiplexer window 名
${status}       - 当前状态
${prev_status}  - 转换前状态
${timestamp}    - 当前时间戳
${backup_dir}   - 备份目录
```

## 任务生命周期

```
                    on_create
                        ↓
┌─────────────────────────────────────────────────────────────┐
│  Pending ──────→ Running ──────→ Review ──────→ Completed   │
│            before_run      before_review    before_complete │
│              after_run      after_review    after_complete  │
│                ↑               │                  │         │
│                └───────────────┘                  │         │
│                  before_resume                    │         │
│                                                   │         │
│                 ←────── before_reset ─────────────┘         │
│                                                   │         │
│                 ←────── before_delete ────────────┘         │
└─────────────────────────────────────────────────────────────┘
```

## 实现方式

原子操作通过 wt 内部子命令实现：
```bash
wt internal git:rebase ${task}
wt internal mux:close-window ${task}
wt internal files:backup ${task}
```

在 hooks 脚本中可以直接调用，wt 会自动注入变量。

## 参考文件

- `src/models/config.rs` - 配置解析
- `src/services/git.rs` - Git 操作
- `src/services/multiplexer/` - Multiplexer 抽象
- `src/commands/` - 各命令实现

## 向后兼容

需要支持旧配置格式，自动迁移到新格式（或至少兼容运行）。
