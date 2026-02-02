# wt - Worktree Task Manager

通过 git worktree 隔离工作区，terminal multiplexer 管理 agent 进程，依赖关系控制任务执行顺序。

## 依赖

- Git (支持 worktree)
- tmux 或 zellij
- Rust (编译安装)

## 安装

```bash
cargo install --path .
```

## 快速开始

```bash
wt init                                    # 初始化
wt create --json '{"name": "auth", "depends": [], "description": "实现认证"}'
wt run auth                                # 启动任务
wt status                                  # 查看状态 (TUI)
wt tail auth                               # 查看最后输出
wt logs                                    # 生成调试日志
wt review auth                             # 标记待审核
wt complete auth                           # 完成任务 (merge + 清理)
wt resume auth                             # 继续开发（从 review 回到 running）
wt reset auth                              # 重置（会备份代码）
```

## 命令

| 命令 | 说明 |
|------|------|
| `wt init` | 初始化配置（自动安装 shell 补全） |
| `wt create --json '{...}'` | 创建任务 |
| `wt validate [name]` | 验证任务 |
| `wt list [--tree] [--json]` | 列出任务（显示索引） |
| `wt next [--json]` | 显示可启动任务 |
| `wt run <name\|index>` | 启动任务（支持名称或索引） |
| `wt run --all` | 启动所有就绪任务 |
| `wt status [--json] [--action X --task Y]` | 查看状态 (默认 TUI) |
| `wt tail <name\|index> [-n N]` | 查看最后 N 条输出 (JSON) |
| `wt logs` | 生成所有任务的过滤日志 |
| `wt review <name\|index>` | 标记待审核 |
| `wt resume <name\|index>` | 继续开发（从 review 回到 running） |
| `wt complete <name\|index>` | 完成任务 (merge + 清理) |
| `wt reset <name\|index>` | 重置到 pending（备份代码）|
| `wt new [name]` | 创建 scratch 环境 |
| `wt delete <name> [--force]` | 删除任务资源 |
| `wt completions generate <shell>` | 生成 shell 补全脚本 |
| `wt completions install` | 安装 shell 补全到配置文件 |

> **提示**：所有接受任务名的命令都支持使用索引，如 `wt run 1` 等同于 `wt run auth`（假设 auth 是第 1 个任务）

## 任务状态

```
○ Pending  →  ● Running  →  ? Review  →  ✓ Completed
  (wt run)    (wt review)   (wt complete)
                  ↑            │
                  └────────────┘  (wt resume)
```

- **review** 标记任务待审核，关闭 multiplexer 窗口
- **resume** 从 Review 恢复到 Running，继续开发
- **complete** 执行 hooks + merge + cleanup
- **reset** 可从任意状态回到 Pending（会备份代码到 `.wt/backups/`）

## Status TUI 快捷键

| 按键 | 功能 |
|------|------|
| `↑↓` / `jk` | 导航 |
| `Enter` | 进入 multiplexer 窗口 |
| `t` | tail (查看输出) |
| `r` | 标记 review |
| `u` | resume (继续开发) |
| `c` | complete (完成) |
| `q` | 退出 |

## Status --action 参数

非交互方式执行 TUI 操作，返回 JSON：

```bash
wt status --action list --task ui      # 查看可用操作
wt status --action review --task ui    # 标记待审核
wt status --action resume --task ui    # 继续开发
wt status --action complete --task ui  # 完成任务
wt status --action enter --task ui     # 获取 multiplexer 命令
wt status --action tail --task ui      # 查看输出
```

## 配置

配置文件位于 `.wt/config.yaml`：

```yaml
# Terminal multiplexer: tmux (默认) 或 zellij
multiplexer: tmux

# Session 名称
session_name: my-project

# Claude CLI 命令（默认: claude）
# claude_command: ccc

# wt run 执行的参数
start_args: --verbose --output-format=stream-json -p "@.wt/tasks/${task}.md 请完成任务"

# 其他可选配置
# worktree_dir: .wt/worktrees
# copy_files:
#   - .env

# 日志过滤 (wt logs)
# logs:
#   exclude_types: [system, progress]
#   exclude_fields: [signature, uuid]

# Hooks - 在任务生命周期各阶段执行脚本
hooks:
  on_create: npm install           # worktree 创建后
  before_review: npm run lint      # review 前检查
  before_complete: npm run build   # 完成前验证
  before_delete: rm -rf node_modules/  # 删除前清理
  before_reset: rm -rf node_modules/   # 重置前清理
```

## Hooks 系统

### 可用 Hooks

| Hook | 触发时机 | 用途 |
|------|----------|------|
| `on_create` | worktree 创建后 | 安装依赖、初始化环境 |
| `before_run` | 启动 agent 前 | 前置检查 |
| `after_run` | agent 启动后 | 通知、日志 |
| `before_review` | 标记 review 前 | lint、测试 |
| `after_review` | 标记 review 后 | 通知 |
| `before_resume` | 恢复开发前 | 环境检查 |
| `before_complete` | 完成任务前 | 构建、最终验证 |
| `after_complete` | 完成任务后 | 清理、通知 |
| `before_delete` | 删除资源前 | 清理大文件 |
| `before_reset` | 重置任务前 | 清理大文件 |

### 变量

Hooks 脚本中可使用以下变量：

| 变量 | 说明 | 示例 |
|------|------|------|
| `${task}` | 任务名 | `auth` |
| `${branch}` | 分支名 | `wt/auth-abc123` |
| `${worktree}` | worktree 路径 | `.wt/worktrees/auth` |
| `${session}` | multiplexer session | `my-project` |
| `${window}` | multiplexer window | `auth` |

### 示例

```yaml
hooks:
  on_create: |
    npm install
    cp .env.example .env

  before_review: |
    npm run lint
    npm run test

  before_complete: |
    npm run build
    npm run test:e2e

  after_complete: |
    wt internal notify "Task Complete" "${task} merged to main"

  before_delete: |
    rm -rf node_modules/ dist/ .next/
```

## 内部操作 (wt internal)

供 hooks 脚本使用的原子操作。

### Git 操作

```bash
wt internal git:fetch <repo_root> <remote>
wt internal git:rebase <worktree_path> <target>
wt internal git:squash-merge <repo_root> <branch>
wt internal git:commit <path> <message>
wt internal git:push <repo_root> <branch> [remote]
wt internal git:has-changes <path>          # exit 0=有变更, 1=无变更
wt internal git:has-conflicts <path>        # exit 0=有冲突, 1=无冲突
wt internal git:stash <path>
wt internal git:stash-pop <path>
wt internal git:create-branch <repo_root> <branch>
wt internal git:delete-branch <repo_root> <branch>
wt internal git:checkout <path> <branch>
wt internal git:current-branch <path>
```

### Multiplexer 操作

```bash
wt internal mux:create-window <session> <window> <cwd> <command>
wt internal mux:close-window <session> <window>
wt internal mux:focus-window <session> <window>
wt internal mux:window-exists <session> <window>  # exit 0=存在, 1=不存在
wt internal mux:send-keys <session> <window> <keys>
wt internal mux:list-windows <session>
```

### 文件操作

```bash
wt internal files:backup <task> [backup_dir]   # 输出备份路径
wt internal files:clean <worktree> <patterns...>
```

### 状态操作

```bash
wt internal status:set <task> <status>    # pending/running/review/completed
wt internal status:get <task>
```

### 任务操作

```bash
wt internal task:exists <task>       # exit 0=存在, 1=不存在
wt internal task:deps-ready <task>   # exit 0=就绪, 1=未就绪
wt internal task:blocked-by <task>   # 输出阻塞的任务列表
```

### 配置操作

```bash
wt internal config:get <key>   # 支持: claude_command, session_name, multiplexer, worktree_dir, start_args
```

### 通知操作

```bash
wt internal notify <title> <message>
wt internal confirm <message>   # exit 0=确认, 1=取消
wt internal abort <message>     # 输出错误并退出
wt internal log <task> <message>
```

## License

MIT
