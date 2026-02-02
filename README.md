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
wt start auth                              # 启动任务
wt status                                  # 查看状态 (TUI)
wt tail auth                               # 查看最后输出
wt logs                                    # 生成调试日志
wt done auth                               # 标记完成
wt merge auth                              # 自动 merge 到 main（rebase + squash）
wt merge auth --agent                      # 静默模式（用于自动化）
wt archive auth                            # 归档（清理 worktree 和分支）
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
| `wt start <name\|index>` | 启动任务（支持名称或索引） |
| `wt start --all` | 启动所有就绪任务 |
| `wt status [--json] [--action X --task Y]` | 查看状态 (默认 TUI) |
| `wt tail <name\|index> [-n N]` | 查看最后 N 条输出 (JSON) |
| `wt logs` | 生成所有任务的过滤日志 |
| `wt done <name\|index>` | 标记完成 |
| `wt merge <name\|index> [--agent]` | 自动 merge 到 main（rebase + squash + archive）|
| `wt archive <name\|index>` | 归档（清理 worktree/分支）|
| `wt reset <name\|index>` | 重置到 pending（备份代码）|
| `wt new [name]` | 创建 scratch 环境 |
| `wt completions generate <shell>` | 生成 shell 补全脚本 |
| `wt completions install` | 安装 shell 补全到配置文件 |

> **提示**：所有接受任务名的命令都支持使用索引，如 `wt start 1` 等同于 `wt start auth`（假设 auth 是第 1 个任务）

## Status TUI 快捷键

| 按键 | 功能 |
|------|------|
| `↑↓` / `jk` | 导航 |
| `Enter` | 进入 multiplexer 窗口 |
| `t` | tail (查看输出) |
| `d` | 标记 done (自动关闭窗口) |
| `m` | 标记 merged |
| `a` | archive (归档) |
| `q` | 退出 |

## Status --action 参数

非交互方式执行 TUI 操作，返回 JSON：

```bash
wt status --action list --task ui      # 查看可用操作
wt status --action done --task ui      # 标记完成
wt status --action merged --task ui    # 标记已合并
wt status --action archive --task ui   # 归档任务
wt status --action enter --task ui     # 获取切换命令
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
# 如果你使用别名，在这里配置
# claude_command: ccc

# wt start 执行的参数
start_args: --verbose --output-format=stream-json -p "@.wt/tasks/${task}.md 请完成任务"

# 其他可选配置
# worktree_dir: .wt/worktrees
# init_script: npm install   # 在 multiplexer 窗口内并行执行
# copy_files:
#   - .env

# 日志过滤 (wt logs)
# logs:
#   exclude_types: [system, progress]
#   exclude_fields: [signature, uuid]

# 归档/重置前的清理脚本
# archive_script: |
#   rm -rf node_modules/
#   rm -rf dist/
```

## 任务状态

```
○ Pending  →  ● Running  →  ✓ Done  →  ☑ Archived
                              ↓
                        (wt merge)
                     自动 merge + archive
```

- **reset** 可从 Running/Done/Archived 回到 Pending（会备份代码到 `.wt/backups/`）
- **merge** 执行 rebase + squash merge + commit，然后自动 archive
- **archive** 执行清理脚本后删除 worktree 和分支

## License

MIT
