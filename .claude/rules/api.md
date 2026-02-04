# 命令参考

## Agent 标记命令

Agent 在 worktree 中执行，通过以下命令标记当前 Step 状态：

```bash
wt step done              # 当前 step → success，触发自动推进
wt step block "原因"       # 当前 step → blocked，等待 human 介入
wt step fail "原因"        # 当前 step → failed，等待 human 介入
```

### Agent 自验证 (Best Practice)

在 agent step 中启用 Stop hook 自验证机制，确保 agent 退出前完成质量检查：

```jsonc
// phase 配置示例
{
  "on_enter": {
    "steps": [
      {
        "agent": {
          "prompt": "@.wt/tasks/${task}.md",
          "settings": ".wt/templates/verify-settings.json"
        },
        "verify": { "run": "true" }
      }
    ]
  }
}
```

**工作流程**：
1. Agent 完成任务尝试退出
2. Stop hook 触发，提示阅读 `.wt/verify.md` 自检清单
3. Agent 根据检查结果调用 `wt step done/block/fail`
4. 状态标记后允许退出

**相关文件**：
- `.wt/hooks/verify-stop.cjs` — Stop hook 脚本
- `.wt/verify.md` — 验证清单模板
- `.wt/templates/verify-settings.json` — Claude settings 模板

## Human 控制命令

人工操作阶段转换：

```bash
wt next <task>            # 推进到下一阶段
wt prev <task>            # 回退到上一阶段
wt stop <task>            # 停止当前进程，保持 phase 不变
wt reset <task>           # 备份代码，重置到 pending
wt delete <task>          # 彻底删除任务
```

### next 行为

1. 停止当前进程 (如果有)
2. 执行当前 phase 的 on_exit workflow
3. 分配下一 phase 的资源 (branch/worktree/window)
4. 更新 phase
5. 执行下一 phase 的 on_enter workflow

### prev 行为

1. 停止当前进程
2. 更新 phase
3. **不执行 on_enter** (回退不触发 workflow)

### stop 行为

1. 向进程发送 Ctrl+C
2. 保留 worktree 和 branch
3. 任务变为 Idle 状态
4. `--kill-window`: 同时关闭 multiplexer 窗口

### reset 行为

1. 备份代码到 `.wt/backups/<task>-<timestamp>/`
2. 删除 worktree 和 branch
3. 重置到 pending (或 `--to <phase>` 指定的阶段)

## 任务管理命令

```bash
wt init                       # 初始化 .wt 目录
wt create --json '{...}'      # 创建任务 (JSON 格式输入)
wt validate [name]            # 验证任务配置和循环依赖
wt list [--tree] [--json]     # 列出任务 (支持树形和 JSON)
wt status [--all] [--json]    # TUI 或 JSON 输出
wt tail <task> [-n N]         # 查看最近 N 条 agent 输出
wt logs                       # 生成调试日志
wt new [name] [--print-path]  # 创建临时 worktree (无任务文件)
```

### status 模式

| 用法 | 说明 |
|------|------|
| `wt status` | TUI 交互界面 |
| `wt status --json` | JSON 输出 (给脚本用) |
| `wt status --json --all` | 包含已完成任务 |
| `wt status --action list --task <name>` | 查询可用操作 |
| `wt status --action enter --task <name>` | 进入任务窗口 |
| `wt status --action next --task <name>` | 推进阶段 |

## 任务索引

所有命令支持 1-based 索引代替任务名：

```bash
wt next 1                 # = wt next <第一个任务>
wt tail 2                 # = wt tail <第二个任务>
```

优先级：任务名 > 索引号

## TUI 快捷键

| 键 | 功能 |
|----|------|
| `j/k` 或 `↑/↓` | 上下选择 |
| `Enter` | 切换到任务的 multiplexer 窗口 |
| `n` | 推进阶段 (next) |
| `p` | 回退阶段 (prev) |
| `s` | 停止进程 (stop) |
| `t` | 查看 transcript (tail) |
| `l` | 打开日志 (logs) |
| `?` | 帮助 |
| `q` | 退出 |

## 配置文件

### .wt/config.jsonc

```jsonc
{
  "multiplexer": "tmux",          // tmux 或 zellij
  "session_name": "project",      // multiplexer session 名
  "claude_command": "claude",     // Claude CLI 路径
  "worktree_dir": ".wt/worktrees",

  // Phase 生命周期
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

### .wt/tasks/*.md

YAML frontmatter + Markdown 描述：

```yaml
---
name: auth
depends:
  - database
---
任务描述...
```

### .wt/status.json

运行时状态存储，包含每个任务的 status/phase/instance 信息。由 wt 自动管理，不要手动编辑。

## 任务名规则

任务名必须是有效的 git 分支名：

- 不能为空
- 不能含空格、制表符
- 不能含 `~ ^ : ? * [ @ {`
- 不能以 `-` 或 `.` 开头
- 不能以 `.` 或 `.lock` 结尾
- 不能含 `..`

## 内部操作 (wt internal)

供脚本和 workflow 调用的原子操作：

```bash
# Git
wt internal git:fetch / git:rebase / git:squash-merge
wt internal git:commit / git:push / git:create-branch / git:delete-branch
wt internal git:has-changes / git:has-conflicts  # exit code 判断
wt internal git:stash / git:stash-pop / git:checkout / git:current-branch

# Multiplexer
wt internal mux:create-window / mux:close-window / mux:focus-window
wt internal mux:window-exists / mux:send-keys / mux:list-windows

# 文件
wt internal files:backup / files:clean
```
