# wt - Worktree Task Manager

多 agent 并行开发任务管理工具。通过 git worktree 隔离工作区、tmux/zellij 管理进程、依赖关系控制执行顺序，让多个 AI agent 同时开发不同功能分支。

## 工作原理

```
                    ┌─ worktree/auth ─── Agent A (developing)
                    │
main repo ──────────┼─ worktree/api  ─── Agent B (reviewing)
                    │
                    └─ worktree/ui   ─── Agent C (developing)
```

每个任务拥有独立的 git worktree 和 multiplexer 窗口，agent 在隔离环境中工作，互不干扰。

## 依赖

- Git (支持 worktree)
- tmux 或 zellij
- Rust toolchain (编译安装)

## 安装

```bash
cargo install --path .
```

## 快速开始

```bash
# 1. 初始化项目
wt init

# 2. 创建任务
wt create --json '{"name": "auth", "depends": [], "description": "实现认证模块"}'
wt create --json '{"name": "api", "depends": ["auth"], "description": "实现 API 层"}'

# 3. 启动任务 (创建 worktree + 启动 agent)
wt next auth

# 4. 查看状态
wt status                  # TUI 交互界面
wt status --json           # JSON 输出

# 5. 监控和控制
wt tail auth               # 查看 agent 最近输出
wt stop auth               # 暂停 agent 进程
wt next auth               # 推进到下一阶段
wt reset auth              # 重置任务 (会备份代码)
```

## 命令速查

### 任务管理

| 命令 | 说明 |
|------|------|
| `wt init` | 初始化 `.wt/` 目录和配置 |
| `wt create --json '{...}'` | 创建任务 |
| `wt validate [name]` | 验证任务配置和循环依赖 |
| `wt list [--tree] [--json]` | 列出所有任务 |
| `wt delete <task> [--force]` | 删除任务及其资源 |

### 阶段控制

| 命令 | 说明 |
|------|------|
| `wt next <task>` | 推进到下一阶段 |
| `wt prev <task>` | 回退到上一阶段 |
| `wt stop <task> [--kill-window]` | 停止进程，保留 worktree |
| `wt reset <task> [--to <phase>]` | 重置任务 (备份后清理) |

### Agent 标记 (agent 在 worktree 内调用)

| 命令 | 说明 |
|------|------|
| `wt step done` | 标记当前 step 成功 |
| `wt step block [reason]` | 标记阻塞，等待人工介入 |
| `wt step fail [reason]` | 标记失败 |

### 状态与日志

| 命令 | 说明 |
|------|------|
| `wt status` | TUI 交互界面 |
| `wt status --json [--all]` | JSON 输出 |
| `wt tail <task> [-n N]` | 最近 N 条 agent 输出 |
| `wt logs` | 生成调试日志 |

### 其他

| 命令 | 说明 |
|------|------|
| `wt new [name] [--print-path]` | 创建临时 worktree (无任务文件) |
| `wt completions install` | 安装 shell 补全 |
| `wt internal <op> <args>` | 供脚本使用的原子操作 |

> 所有接受任务名的命令都支持 1-based 索引：`wt next 1` = 第一个任务

## 状态模型

wt 用两个维度描述任务状态：

- **Status** — 资源状态 (是否有进程在运行)
- **Phase** — 业务阶段 (任务进展到哪一步)

```
Status:  ○ Pending  →  ● Active  ⇄  ◐ Idle  →  ✓ Completed
Phase:   (none)     →  developing → reviewing →  (done)
```

### Status x Phase 组合

| Status | Phase | 场景 |
|--------|-------|------|
| Pending | (none) | 任务已定义，未创建资源 |
| Active | developing | agent 正在编码 |
| Idle | developing | agent 暂停，等待用户 |
| Active | reviewing | review 进行中 |
| Idle | reviewing | review 完成，等待下一步 |
| Completed | (none) | 任务完成 |

### Idle 原因

| 原因 | 说明 |
|------|------|
| `done` | 当前阶段正常完成 |
| `human_review` | 等待人工审核 |
| `error` | 命令执行出错 |
| `conflict` | 合并冲突待解决 |
| `timeout` | 执行超时 |
| `manual` | 用户手动暂停 |

## TUI 快捷键

| 键 | 功能 |
|----|------|
| `j/k` 或 `↑/↓` | 上下导航 |
| `Enter` | 进入任务的 multiplexer 窗口 |
| `n` | 推进阶段 (next) |
| `p` | 回退阶段 (prev) |
| `s` | 停止进程 (stop) |
| `t` | 查看 transcript (tail) |
| `l` | 打开日志 (logs) |
| `?` | 帮助 |
| `q` | 退出 |

## 配置

配置文件 `.wt/config.jsonc` (JSONC 格式，支持注释)：

```jsonc
{
  "multiplexer": "tmux",              // tmux 或 zellij
  "session_name": "my-project",
  "claude_command": "claude",
  "worktree_dir": ".wt/worktrees",

  // 阶段定义
  "phases": {
    "sequence": ["pending", "developing", "reviewing", "completed"],
    "definitions": {
      "developing": {
        "id": "developing",
        "resources": { "branch": true, "worktree": true, "window": true },
        "on_enter": {
          "steps": [{
            "agent": { "prompt": "@.wt/tasks/${task}.md" },
            "verify": { "run": "true" }
          }]
        }
      },
      "reviewing": {
        "id": "reviewing",
        "resources": { "branch": true }
      },
      "completed": {
        "id": "completed",
        "terminal": true
      }
    }
  }
}
```

### 任务文件 (.wt/tasks/*.md)

```yaml
---
name: auth
depends:
  - database
---

实现 JWT 认证模块...
```

## 内部操作

`wt internal` 提供供脚本和 workflow 调用的原子操作：

```bash
# Git 操作
wt internal git:fetch <repo> <remote>
wt internal git:rebase <worktree> <target>
wt internal git:squash-merge <repo> <branch>
wt internal git:has-changes <path>          # exit 0=有变更
wt internal git:has-conflicts <path>        # exit 0=有冲突
wt internal git:create-branch <repo> <branch>

# Multiplexer 操作
wt internal mux:create-window <session> <window> <cwd> <command>
wt internal mux:close-window <session> <window>
wt internal mux:focus-window <session> <window>
wt internal mux:window-exists <session> <window>
wt internal mux:send-keys <session> <window> <keys>

# 文件操作
wt internal files:backup <task> [backup_dir]
wt internal files:clean <worktree> <patterns...>
```

## License

MIT
