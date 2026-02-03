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
wt init                    # 初始化
wt create --json '{"name": "auth", "depends": [], "description": "实现认证"}'
wt next auth               # 推进任务 (Pending → Active)
wt status                  # 查看状态 (TUI)
wt tail auth               # 查看最后输出
wt stop auth               # 停止任务进程
wt reset auth              # 重置（会备份代码）
wt delete auth             # 删除任务资源
```

## 命令

### 任务管理

| 命令 | 说明 |
|------|------|
| `wt init` | 初始化配置（自动安装 shell 补全） |
| `wt create --json '{...}'` | 创建任务 |
| `wt validate [name]` | 验证任务 |
| `wt list [--tree] [--json]` | 列出任务（显示索引） |
| `wt delete <name> [--force]` | 删除任务资源 |

### 阶段控制

| 命令 | 说明 |
|------|------|
| `wt next <task>` | 推进到下一阶段 |
| `wt prev <task>` | 回退到上一阶段 |
| `wt stop <task>` | 停止任务进程 |
| `wt reset <task> [--to <phase>]` | 重置任务（可指定目标阶段） |
| `wt step done\|block\|fail` | Agent 标记 step 状态 |

### 状态和日志

| 命令 | 说明 |
|------|------|
| `wt status [--json]` | 查看状态 (默认 TUI) |
| `wt tail <name> [-n N]` | 查看最后 N 条输出 |
| `wt logs` | 生成所有任务的调试日志 |

### 其他

| 命令 | 说明 |
|------|------|
| `wt new [name]` | 创建 scratch 环境 |
| `wt completions generate <shell>` | 生成 shell 补全脚本 |
| `wt completions install` | 安装 shell 补全 |

> **提示**：所有接受任务名的命令都支持使用索引，如 `wt next 1`

## 任务状态

### 状态模型

wt 使用两个维度描述任务状态：

- **Status** - 资源状态（是否有进程在运行）
- **Phase** - 业务阶段（开发进度）

```
Status:
○ Pending  →  ● Active  ⇄  ◐ Idle  →  ✓ Completed
  (未创建)    (有进程)    (无进程)    (已完成)

Phase:
(none) → developing → reviewing → merging → (done)
```

### Status × Phase 组合

| Status | Phase | 场景 |
|--------|-------|------|
| Pending | (none) | 任务已定义，未创建资源 |
| Active | developing | agent 正在开发 |
| Idle | developing | agent 暂停，等待用户 |
| Active | reviewing | review 进行中 |
| Idle | reviewing | review 完成，等待下一步 |
| Active | merging | 合并/清理进行中 |
| Completed | (none) | 任务完成 |

### Idle 原因

当任务处于 Idle 状态时，`idle_reason` 说明原因：

| 原因 | 说明 |
|------|------|
| `done` | 当前阶段正常完成 |
| `human_review` | 等待人工审核 |
| `error` | 命令执行出错 |
| `conflict` | 合并冲突待解决 |
| `timeout` | 执行超时 |
| `manual` | 用户手动暂停 |

## Status TUI 快捷键

| 按键 | 功能 |
|------|------|
| `↑↓` / `jk` | 导航 |
| `Enter` | 进入 multiplexer 窗口 |
| `n` | next (推进阶段) |
| `p` | prev (回退阶段) |
| `s` | stop (停止进程) |
| `t` | tail (查看输出) |
| `l` | logs (打开日志) |
| `?` | 帮助 |
| `q` | 退出 |

## 配置

配置文件位于 `.wt/config.jsonc`（JSONC 格式，支持注释）：

```jsonc
{
  // Terminal multiplexer: tmux (默认) 或 zellij
  "multiplexer": "tmux",

  // Session 名称
  "session_name": "my-project",

  // Claude CLI 命令（默认: claude）
  "claude_command": "claude",

  // Worktree 目录
  "worktree_dir": ".wt/worktrees"
}
```

## 内部操作 (wt internal)

供脚本使用的原子操作。

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

## License

MIT
