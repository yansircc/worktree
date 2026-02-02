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
wt resume auth                             # 继续开发
wt pause auth                              # 暂停任务
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
| `wt resume <name\|index>` | 继续开发 |
| `wt pause <name\|index> [--reason R]` | 暂停任务 |
| `wt complete <name\|index>` | 完成任务 (merge + 清理) |
| `wt reset <name\|index>` | 重置到 pending（备份代码）|
| `wt new [name]` | 创建 scratch 环境 |
| `wt delete <name> [--force]` | 删除任务资源 |
| `wt hooks list` | 列出配置的 hooks |
| `wt hooks run <hook> [--task T]` | 手动触发 hook（调试用） |
| `wt completions generate <shell>` | 生成 shell 补全脚本 |
| `wt completions install` | 安装 shell 补全到配置文件 |

> **提示**：所有接受任务名的命令都支持使用索引，如 `wt run 1` 等同于 `wt run auth`（假设 auth 是第 1 个任务）

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
| Active | reviewing | review pipeline 在运行 |
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
  "worktree_dir": ".wt/worktrees",

  // Hooks - 每个命令的行为定义
  "hooks": {
    "run": [
      { "type": "script", "run": "npm install" },
      { "type": "agent", "interactive": true, "prompt": "..." }
    ],
    "review": [
      { "type": "script", "run": "npm run lint" }
    ]
  }
}
```

完整示例见 `.wt/config.example.jsonc`。

## Hooks 系统

### 设计原则

- **命令 = Hook** - 每个命令的行为完全由 hooks 定义
- **全部 Hooks 化** - 没有特殊配置，全部统一为 hooks
- **Pipeline 优先** - 多 agent 通过 stream-json 自动串联

### 可用 Hooks

| Hook | 触发命令 | 默认状态转换 |
|------|----------|--------------|
| `run` | `wt run` | Pending → Active + developing |
| `review` | `wt review` | * → Idle + reviewing |
| `resume` | `wt resume` | Idle → Active |
| `complete` | `wt complete` | * → Completed |
| `delete` | `wt delete` | 移除记录 |
| `reset` | `wt reset` | * → Pending |

### Step 类型

#### 1. script

执行 shell 脚本：

```jsonc
{
  "type": "script",
  "run": "npm run lint",
  "on_error": { ... }  // 可选：失败时执行的步骤
}
```

#### 2. agent

运行 Claude agent：

```jsonc
{
  "type": "agent",
  "interactive": false,           // false = -p 模式, true = REPL 模式
  "model": "haiku",               // haiku | sonnet | opus
  "prompt": "...",                // 内联 prompt 或 @file 引用
  "tools": ["Read", "Edit"],      // 可用工具列表
  "allowed_tools": ["Bash(npm *)"], // 自动批准的工具
  "skip_permissions": false,      // 是否跳过权限提示
  "output_format": "text",        // text | json | stream-json
  "window": "new"                 // 交互模式: main | new
}
```

#### 3. internal

调用 wt 内置操作：

```jsonc
{
  "type": "internal",
  "run": "worktree:create",
  "on_conflict": { ... }  // 可选：冲突时执行的步骤
}
```

可用操作：`worktree:create`, `worktree:destroy`, `branch:create`, `branch:delete`, `branch:merge`, `window:create`, `window:close`, `files:backup`, `files:clean`

#### 4. condition

条件判断：

```jsonc
{
  "type": "condition",
  "if": "wt internal git:has-changes ${worktree}",
  "then": { ... },
  "else": { ... }
}
```

### Pipeline 模式

多个 agent 通过 stream-json 自动串联：

```jsonc
{
  "hooks": {
    "review": {
      "pipeline": [
        {
          "type": "agent",
          "model": "haiku",
          "prompt": "List all changed files and summarize changes"
        },
        {
          "type": "agent",
          "model": "sonnet",
          "prompt": "Based on the above, perform detailed code review"
        }
      ]
    }
  }
}
```

wt 自动转换为：

```bash
claude -p --output-format stream-json "prompt1" | \
claude -p --input-format stream-json --output-format stream-json "prompt2"
```

### 变量

所有 step 中可使用：

| 变量 | 说明 |
|------|------|
| `${task}` | 任务名 |
| `${branch}` | 分支名 |
| `${worktree}` | worktree 路径 |
| `${session}` | multiplexer session |
| `${window}` | multiplexer window |
| `${repo_root}` | 仓库根目录 |
| `${phase}` | 当前阶段 |

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
wt internal status:set <task> <status>    # pending/active/idle/completed
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
wt internal config:get <key>   # 支持: claude_command, session_name, multiplexer, worktree_dir
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
